import Foundation

/// Windows credential and config roots. Helpers are compiled on every platform so Linux tests can
/// pin APPDATA/USERPROFILE without requiring a Windows host.
public enum WindowsApplicationPaths {
    public static func roamingAppData(
        home: String,
        environment: [String: String] = ProcessInfo.processInfo.environment) -> String
    {
        if let appData = self.absoluteEnvironmentValue(environment["APPDATA"]) {
            return appData
        }
        return self.join(home, "AppData", "Roaming")
    }

    public static func localAppData(
        home: String,
        environment: [String: String] = ProcessInfo.processInfo.environment) -> String
    {
        if let local = self.absoluteEnvironmentValue(environment["LOCALAPPDATA"]) {
            return local
        }
        return self.join(home, "AppData", "Local")
    }

    public static func userProfile(
        home: String? = nil,
        environment: [String: String] = ProcessInfo.processInfo.environment,
        fileManager: FileManager = .default) -> String
    {
        if let home {
            return home
        }
        if let profile = self.absoluteEnvironmentValue(environment["USERPROFILE"]) {
            return profile
        }
        if let homeEnv = self.absoluteEnvironmentValue(environment["HOME"]) {
            return homeEnv
        }
        return fileManager.homeDirectoryForCurrentUser.path
    }

    public static func cursorUserDatabase(
        home: String? = nil,
        environment: [String: String] = ProcessInfo.processInfo.environment) -> String
    {
        let profile = self.userProfile(home: home, environment: environment)
        return self.join(
            self.roamingAppData(home: profile, environment: environment),
            "Cursor",
            "User",
            "globalStorage",
            "state.vscdb")
    }

    public static func cursorAgentAuth(
        home: String? = nil,
        environment: [String: String] = ProcessInfo.processInfo.environment) -> String
    {
        let profile = self.userProfile(home: home, environment: environment)
        return self.join(
            self.roamingAppData(home: profile, environment: environment),
            "cursor",
            "auth.json")
    }

    public static func configFileURL(
        home: URL,
        environment: [String: String] = ProcessInfo.processInfo.environment) -> URL
    {
        let profile = home.path
        let roamingPath = self.roamingAppData(home: profile, environment: environment)
        let roaming = URL(fileURLWithPath: roamingPath, isDirectory: true)
        return roaming
            .appendingPathComponent("codexbar", isDirectory: true)
            .appendingPathComponent("config.json")
    }

    public static var environmentPathSeparator: Character {
        #if os(Windows)
        ";"
        #else
        ":"
        #endif
    }

    public static func isAbsolutePath(_ path: String) -> Bool {
        if path.isEmpty { return false }
        if path.hasPrefix("~") { return false }
        if path.hasPrefix("/") { return true }
        if path.hasPrefix(#"\\"#) { return true }
        let utf16 = Array(path.utf16)
        if utf16.count >= 3,
           let first = utf16.first,
           (65...90).contains(first) || (97...122).contains(first),
           utf16[1] == 58,
           utf16[2] == 92 || utf16[2] == 47
        {
            return true
        }
        return false
    }

    public static func executableCandidates(named name: String) -> [String] {
        if name.contains(".") { return [name] }
        return [name, "\(name).exe", "\(name).cmd", "\(name).bat"]
    }

    static func join(_ components: String...) -> String {
        components.reduce(into: "") { result, component in
            if result.isEmpty {
                result = component
                return
            }
            if result.hasSuffix("/") || result.hasSuffix("\\") {
                result += component
            } else {
                result += "/" + component
            }
        }
    }

    private static func absoluteEnvironmentValue(_ raw: String?) -> String? {
        guard let trimmed = raw?.trimmingCharacters(in: .whitespacesAndNewlines),
              !trimmed.isEmpty,
              self.isAbsolutePath(trimmed)
        else {
            return nil
        }
        return trimmed
    }
}

#if os(Windows)
@_exported import CWindowsPOSIX
#endif
