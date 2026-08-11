import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.kde.plasma.components as PlasmaComponents

Rectangle {
    id: card
    required property string providerName
    required property color accentColor
    property var status: null
    property string healthValue: "unavailable"
    property string freshnessValue: "unknown"
    property string planValue: "Plan unavailable"
    property var remaining: null
    property string resetText: "Not available"
    readonly property string healthLabel: healthValue.replaceAll("_", " ")
    readonly property string healthIcon: healthValue === "healthy" ? "✓" : healthValue === "error" || healthValue === "unsupported" || healthValue === "source_changed" ? "×" : "!"
    readonly property real amount: remaining === null || remaining === undefined ? 0 : Number(remaining)
    radius: Kirigami.Units.largeSpacing
    color: Qt.tint(Kirigami.Theme.backgroundColor, Qt.alpha(accentColor, 0.09))
    border.color: Qt.alpha(accentColor, 0.42)
    Rectangle { anchors.left: parent.left; anchors.right: parent.right; anchors.top: parent.top; height: 3; radius: parent.radius; color: card.accentColor }
    ColumnLayout {
        anchors.fill: parent; anchors.margins: Kirigami.Units.largeSpacing * 1.25; spacing: Kirigami.Units.smallSpacing
        RowLayout {
            Layout.fillWidth: true
            ColumnLayout { spacing: 0
                PlasmaComponents.Label { text: card.providerName; font.pixelSize: Kirigami.Theme.defaultFont.pixelSize * 1.2; font.bold: true }
                PlasmaComponents.Label { text: card.planValue; color: Kirigami.Theme.disabledTextColor; font.pixelSize: Kirigami.Theme.smallFont.pixelSize }
            }
            Item { Layout.fillWidth: true }
            Rectangle {
                implicitWidth: healthText.implicitWidth + Kirigami.Units.largeSpacing; implicitHeight: healthText.implicitHeight + Kirigami.Units.smallSpacing
                radius: height / 2; color: Qt.alpha(card.accentColor, 0.16); border.color: Qt.alpha(card.accentColor, 0.4)
                PlasmaComponents.Label { id: healthText; anchors.centerIn: parent; text: card.healthIcon + " " + card.healthLabel.charAt(0).toUpperCase() + card.healthLabel.slice(1); color: card.accentColor; font.pixelSize: Kirigami.Theme.smallFont.pixelSize; font.bold: true }
            }
        }
        Item { Layout.preferredHeight: Kirigami.Units.smallSpacing }
        RowLayout { spacing: Kirigami.Units.smallSpacing
            PlasmaComponents.Label { text: card.remaining === null || card.remaining === undefined ? "—" : Math.round(card.amount) + "%"; font.pixelSize: Kirigami.Theme.defaultFont.pixelSize * 2.5; font.bold: true }
            PlasmaComponents.Label { text: "remaining"; color: Kirigami.Theme.disabledTextColor; Layout.alignment: Qt.AlignBottom; Layout.bottomMargin: Kirigami.Units.smallSpacing }
        }
        Rectangle { Layout.fillWidth: true; implicitHeight: 6; radius: 3; color: Qt.alpha(Kirigami.Theme.textColor, 0.12)
            Rectangle { width: parent.width * Math.max(0, Math.min(100, card.amount)) / 100; height: parent.height; radius: parent.radius; color: card.accentColor }
        }
        Item { Layout.fillHeight: true }
        GridLayout { columns: 2; columnSpacing: Kirigami.Units.largeSpacing; rowSpacing: Kirigami.Units.smallSpacing
            PlasmaComponents.Label { text: "Resets"; color: Kirigami.Theme.disabledTextColor }
            PlasmaComponents.Label { text: card.resetText; Layout.fillWidth: true; elide: Text.ElideRight }
            PlasmaComponents.Label { text: "Freshness"; color: Kirigami.Theme.disabledTextColor }
            PlasmaComponents.Label { text: card.freshnessValue; color: card.accentColor; font.bold: true }
        }
    }
}
