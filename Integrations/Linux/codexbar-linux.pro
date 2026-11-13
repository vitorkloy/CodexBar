QT += core gui widgets quick qml quickcontrols2 network svg
unix:!macx: QT += dbus
CONFIG += c++17 console
CONFIG -= app_bundle
win32 {
    CONFIG -= console
    CONFIG += windows
    TARGET = codexbar-windows
} else {
    TARGET = codexbar-linux
}
SOURCES += main.cpp DesktopController.cpp
HEADERS += DesktopController.h
RESOURCES += desktop.qrc provider_icons
provider_icons.files = $$files($$PWD/../../Sources/CodexBar/Resources/ProviderIcon-*.svg)
provider_icons.prefix = /logos
provider_icons.base = $$PWD/../../Sources/CodexBar/Resources
!win32: QMAKE_CXXFLAGS += -Wall -Wextra
win32: QMAKE_CXXFLAGS += /utf-8
target.path = $$PREFIX/bin
isEmpty(PREFIX): target.path = /usr/local/bin
INSTALLS += target

DESKTOP_VERSION = $$(CODEXBAR_DESKTOP_VERSION)
isEmpty(DESKTOP_VERSION): DESKTOP_VERSION = 0.1.0
DEFINES += CODEXBAR_DESKTOP_VERSION=\\\"$$DESKTOP_VERSION\\\"
