import QtQuick
import "."
Item {
    id:page
    property var manager
    property var surface
    property alias inputRegion: card
    Rectangle {
        id:card; width:250; height:160; anchors.centerIn:parent; radius:8; color:"#33475f"
        Column { anchors.centerIn:parent; spacing:20
            Text { text:"Only this card takes input"; color:"white"; font.pixelSize:15 }
            Action { text:"Close"; onClicked: { console.log("INPUT_CLICK " + page.surface.instanceId); page.surface.shown=false; } }
        }
    }
}
