import Foundation
import Testing
@testable import CodexBarCore

struct WindowsApplicationPathsTests {
    @Test
    func `roaming app data prefers absolute APPDATA`() {
        let path = WindowsApplicationPaths.roamingAppData(
            home: #"C:\Users\test"#,
            environment: ["APPDATA": #"C:\Users\test\AppData\Roaming"#])
        #expect(path == #"C:\Users\test\AppData\Roaming"#)
    }

    @Test
    func `roaming app data falls back beside the profile`() {
        let path = WindowsApplicationPaths.roamingAppData(
            home: #"C:\Users\test"#,
            environment: ["APPDATA": "relative/appdata"])
        #expect(path == #"C:\Users\test/AppData/Roaming"#)
    }

    @Test
    func `cursor database uses roaming Cursor globalStorage`() {
        let path = WindowsApplicationPaths.cursorUserDatabase(
            home: #"C:\Users\test"#,
            environment: ["APPDATA": #"C:\Users\test\AppData\Roaming"#])
        #expect(path == #"C:\Users\test\AppData\Roaming/Cursor/User/globalStorage/state.vscdb"#)
    }

    @Test
    func `cursor-agent auth follows roaming config`() {
        let path = WindowsApplicationPaths.cursorAgentAuth(
            home: #"C:\Users\test"#,
            environment: ["APPDATA": #"D:\config"#])
        #expect(path == #"D:\config/cursor/auth.json"#)
    }

    @Test
    func `config file prefers APPDATA codexbar`() {
        let url = WindowsApplicationPaths.configFileURL(
            home: URL(fileURLWithPath: #"C:\Users\test"#, isDirectory: true),
            environment: ["APPDATA": #"C:\Users\test\AppData\Roaming"#])
        #expect(url.path.contains("codexbar"))
        #expect(url.lastPathComponent == "config.json")
    }

    @Test(arguments: [
        (#"C:\Users\test"#, true),
        (#"D:/data"#, true),
        (#"\\server\share"#, true),
        ("/home/test", true),
        ("relative/path", false),
        ("~/config", false),
        ("", false),
    ])
    func `absolute path detection`(path: String, expected: Bool) {
        #expect(WindowsApplicationPaths.isAbsolutePath(path) == expected)
    }

    @Test
    func `executable candidates add Windows suffixes`() {
        #expect(WindowsApplicationPaths.executableCandidates(named: "claude") == [
            "claude", "claude.exe", "claude.cmd", "claude.bat",
        ])
        #expect(WindowsApplicationPaths.executableCandidates(named: "codex.exe") == ["codex.exe"])
    }

    @Test
    func `local app data prefers absolute LOCALAPPDATA`() {
        let path = WindowsApplicationPaths.localAppData(
            home: #"C:\Users\test"#,
            environment: ["LOCALAPPDATA": #"C:\Users\test\AppData\Local"#])
        #expect(path == #"C:\Users\test\AppData\Local"#)
    }

    @Test
    func `user profile prefers USERPROFILE`() {
        let path = WindowsApplicationPaths.userProfile(
            environment: ["USERPROFILE": #"D:\Users\codex"#],
            fileManager: .default)
        #expect(path == #"D:\Users\codex"#)
    }

    @Test
    func `claude home prefers USERPROFILE when HOME is absent`() {
        let url = ClaudeConfigPaths.homeDirectory(
            environment: ["USERPROFILE": #"C:\Users\claude"#])
        #expect(WindowsApplicationPaths.isAbsolutePath(url.path) || url.path.contains("claude"))
        #expect(url.path.contains("claude"))
    }
}
