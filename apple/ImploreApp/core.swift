import App
import Foundation
import Shared

@MainActor
class Core: ObservableObject {
    @Published var view: ViewModel

    private var core: CoreFFI
    private let keyValueStore = KeyValueStore()

    init() {
        self.core = CoreFFI()
        // swiftlint:disable:next force_try
        self.view = try! .bincodeDeserialize(input: [UInt8](core.view()))
        LocalTimeSync.sync(to: self)
        update(.restore)
    }

    func update(_ event: Event) {
        // swiftlint:disable:next force_try
        let effects = [UInt8](core.update(data: Data(try! event.bincodeSerialize())))
        processEffects(effects)
    }

    private func resolve(requestId: UInt32, response: KeyValueResult) {
        // swiftlint:disable:next force_try
        let effects = [UInt8](core.resolve(id: requestId, data: Data(try! response.bincodeSerialize())))
        processEffects(effects)
    }

    private func resolveHttp(requestId: UInt32, response: HttpResult) {
        // swiftlint:disable:next force_try
        let effects = [UInt8](core.resolve(id: requestId, data: Data(try! response.bincodeSerialize())))
        processEffects(effects)
    }

    private func processEffects(_ effects: [UInt8]) {
        // swiftlint:disable:next force_try
        let requests = try! Requests.bincodeDeserialize(input: effects).value
        for request in requests {
            processEffect(request)
        }
    }

    func processEffect(_ request: Request) {
        switch request.effect {
        case .render:
            // swiftlint:disable:next force_try
            view = try! .bincodeDeserialize(input: [UInt8](core.view()))
        case let .keyValue(operation):
            let result = processKeyValue(operation)
            resolve(requestId: request.id, response: result)
        case let .http(httpRequest):
            let requestId = request.id
            Task {
                let result = await Self.performHttp(httpRequest)
                self.resolveHttp(requestId: requestId, response: result)
            }
        }
    }

    private static func performHttp(_ request: HttpRequest) async -> HttpResult {
        guard let url = URL(string: request.url) else {
            return .err(.url(request.url))
        }

        var urlRequest = URLRequest(url: url)
        urlRequest.httpMethod = request.method
        urlRequest.cachePolicy = .reloadIgnoringLocalCacheData
        urlRequest.timeoutInterval = 20
        urlRequest.httpShouldHandleCookies = false
        for header in request.headers {
            urlRequest.setValue(header.value, forHTTPHeaderField: header.name)
        }
        if !request.body.isEmpty {
            urlRequest.httpBody = Data(request.body)
        }

        do {
            let (data, response) = try await URLSession.shared.data(for: urlRequest)
            let status: UInt16
            var headers: [HttpHeader] = []
            if let http = response as? HTTPURLResponse {
                status = UInt16(http.statusCode)
                for (key, value) in http.allHeaderFields {
                    guard let name = key as? String, let stringValue = value as? String else {
                        continue
                    }
                    headers.append(HttpHeader(name: name, value: stringValue))
                }
            } else {
                status = 200
            }
            return .ok(HttpResponse(status: status, headers: headers, body: [UInt8](data)))
        } catch {
            return .err(.io(error.localizedDescription))
        }
    }

    private func processKeyValue(_ operation: KeyValueOperation) -> KeyValueResult {
        switch operation {
        case let .get(key):
            if let value = keyValueStore.get(key: key) {
                return .ok(response: .get(value: .bytes([UInt8](value))))
            }
            return .ok(response: .get(value: .none))

        case let .set(key, value):
            let previous = keyValueStore.get(key: key)
            keyValueStore.set(key: key, value: Data(value))
            if let previous {
                return .ok(response: .set(previous: .bytes([UInt8](previous))))
            }
            return .ok(response: .set(previous: .none))

        case let .delete(key):
            let previous = keyValueStore.get(key: key)
            keyValueStore.delete(key: key)
            if let previous {
                return .ok(response: .delete(previous: .bytes([UInt8](previous))))
            }
            return .ok(response: .delete(previous: .none))

        case let .exists(key):
            return .ok(response: .exists(isPresent: keyValueStore.exists(key: key)))

        case let .listKeys(prefix, cursor):
            let keys = keyValueStore.listKeys(prefix: prefix, cursor: cursor)
            return .ok(response: .listKeys(keys: keys, nextCursor: 0))
        }
    }
}
