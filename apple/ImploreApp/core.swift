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
