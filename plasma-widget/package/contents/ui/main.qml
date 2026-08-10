import QtQuick
import QtQuick.Layouts
import QtCore
import org.kde.kirigami as Kirigami
import org.kde.plasma.components as PlasmaComponents
import org.kde.plasma.plasma5support as Plasma5Support
import org.kde.plasma.plasmoid

PlasmoidItem {
    id: root
    readonly property url homeUrl: StandardPaths.writableLocation(StandardPaths.HomeLocation)
    readonly property string helperPath: homeUrl.toLocalFile() + "/.local/libexec/sagewatch-plasma-provider"
    readonly property string refreshHelperPath: homeUrl.toLocalFile() + "/.local/libexec/sagewatch-plasma-refresh"
    property var snapshot: ({"providers": {}})
    property string errorMessage: ""
    property bool refreshing: false

    Plasmoid.icon: "view-statistics"
    Plasmoid.title: "Sagewatch"
    preferredRepresentation: fullRepresentation

    function provider(key) {
        var state = snapshot.providers ? snapshot.providers[key] : null
        return state && state.status ? state.status : null
    }
    function percentage(status) {
        var item = headline(status)
        return item ? item.remaining_percent : null
    }
    function headline(status) {
        if (!status || !status.windows || !status.headline_window_id) return null
        for (var i = 0; i < status.windows.length; i++)
            if (status.windows[i].id === status.headline_window_id) return status.windows[i]
        return null
    }
    function resetLabel(status) {
        var item = headline(status)
        if (!item || !item.reset_at) return "Not available"
        var value = new Date(item.reset_at)
        return isNaN(value.getTime()) ? "Not available" : value.toLocaleString(Qt.locale(), Locale.ShortFormat)
    }
    function run(path) {
        if (dataSource.connectedSources.length) dataSource.disconnectSource(dataSource.connectedSources[0])
        dataSource.connectSource(path)
    }
    function refresh() { refreshing = true; run(refreshHelperPath) }
    Component.onCompleted: run(helperPath)

    Timer { interval: 60000; repeat: true; running: true; onTriggered: root.run(root.helperPath) }
    Plasma5Support.DataSource {
        id: dataSource
        engine: "executable"
        onNewData: function(sourceName, data) {
            var stdout = data.stdout ? String(data.stdout).trim() : ""
            var stderr = data.stderr ? String(data.stderr).trim() : ""
            var hasExitCode = data["exit code"] !== undefined && data["exit code"] !== null
            if (!stdout && !stderr && !hasExitCode) return
            root.refreshing = false
            if (stdout) try { root.snapshot = JSON.parse(stdout); root.errorMessage = "" }
            catch (error) { root.errorMessage = "Sagewatch returned invalid local data." }
            else if (stderr) root.errorMessage = stderr
            else if (Number(data["exit code"]) !== 0) root.errorMessage = "Sagewatch data helper exited with code " + data["exit code"] + "."
            disconnectSource(sourceName)
        }
    }

    fullRepresentation: Item {
        Layout.minimumWidth: Kirigami.Units.gridUnit * 22
        Layout.minimumHeight: Kirigami.Units.gridUnit * 15
        Layout.preferredWidth: Kirigami.Units.gridUnit * 31
        Layout.preferredHeight: Kirigami.Units.gridUnit * 18
        Rectangle {
            anchors.fill: parent
            radius: Kirigami.Units.largeSpacing * 1.5
            color: Kirigami.Theme.backgroundColor
            border.color: Qt.alpha(Kirigami.Theme.textColor, 0.14)
            ColumnLayout {
                anchors.fill: parent
                anchors.margins: Kirigami.Units.largeSpacing * 1.5
                spacing: Kirigami.Units.largeSpacing
                RowLayout {
                    Layout.fillWidth: true
                    ColumnLayout {
                        spacing: 1
                        PlasmaComponents.Label { text: "LOCAL ALLOWANCE MONITOR"; color: Kirigami.Theme.disabledTextColor; font.pixelSize: Kirigami.Theme.smallFont.pixelSize; font.bold: true; font.letterSpacing: 1.4 }
                        PlasmaComponents.Label { text: "Sagewatch"; font.pixelSize: Kirigami.Theme.defaultFont.pixelSize * 1.65; font.bold: true }
                    }
                    Item { Layout.fillWidth: true }
                    PlasmaComponents.ToolButton {
                        icon.name: "view-refresh"; text: root.refreshing ? "Refreshing" : "Refresh"
                        display: PlasmaComponents.AbstractButton.IconOnly; enabled: !root.refreshing
                        onClicked: root.refresh(); PlasmaComponents.ToolTip.text: text; PlasmaComponents.ToolTip.visible: hovered
                    }
                }
                PlasmaComponents.Label { visible: root.errorMessage.length > 0; Layout.fillWidth: true; text: root.errorMessage; color: Kirigami.Theme.negativeTextColor; wrapMode: Text.Wrap }
                RowLayout {
                    Layout.fillWidth: true; Layout.fillHeight: true; spacing: Kirigami.Units.largeSpacing
                    ProviderCard { Layout.fillWidth: true; Layout.fillHeight: true; providerName: "Claude"; accentColor: "#c87550"; status: root.provider("claude"); remaining: root.percentage(status); resetText: root.resetLabel(status) }
                    ProviderCard { Layout.fillWidth: true; Layout.fillHeight: true; providerName: "Codex"; accentColor: "#3d9b70"; status: root.provider("codex"); remaining: root.percentage(status); resetText: root.resetLabel(status) }
                }
                PlasmaComponents.Label { Layout.fillWidth: true; horizontalAlignment: Text.AlignHCenter; text: "Local only · No credentials stored"; color: Kirigami.Theme.disabledTextColor; font.pixelSize: Kirigami.Theme.smallFont.pixelSize }
            }
        }
    }
}
