import QtQuick
import "."
import Quickshell
Rectangle {
    id: page
    property var manager
    property var surface
    color:"#202332"
    Row {
        anchors.left:parent.left; anchors.leftMargin:14; anchors.verticalCenter:parent.verticalCenter; spacing:8
        Action { text:"Apps"; onClicked:page.manager.toggle('launcher',page.surface.screen.name) }
        Action { text:"Menu"; onClicked:page.manager.toggle('menu',page.surface.screen.name) }
        Repeater {
            model: page.manager ? page.manager.workspaces : []
            Action {
                required property var modelData
                text:String(modelData.id)
                selected:page.manager.currentWorkspace===String(modelData.id)
                onClicked:page.manager.activate(modelData.id)
            }
        }
    }
    Text { anchors.centerIn:parent; text:"Shell runtime"; color:"#eeeeff"; font.pixelSize:14 }
    Text { anchors.right:parent.right; anchors.rightMargin:16; anchors.verticalCenter:parent.verticalCenter; text:Qt.formatDateTime(clock.date,"hh:mm:ss"); color:"#eeeeff"; font.pixelSize:14 }
    SystemClock { id:clock; precision:SystemClock.Seconds }
}
