import MacCommandBarKit
import SwiftUI

@main
struct MacCommandBarApp: App {
    @StateObject private var state = AppState.bootstrap()

    var body: some Scene {
        MenuBarExtra("Assembly", systemImage: "command") {
            CommandCenterView()
                .environmentObject(state)
        }
        .menuBarExtraStyle(.window)

        Settings {
            SettingsView()
                .environmentObject(state)
        }
    }
}
