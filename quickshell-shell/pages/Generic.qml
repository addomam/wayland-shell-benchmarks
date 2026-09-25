import QtQuick
Rectangle {
    property var manager
    property var surface
    color:"#304254"
    Text { anchors.centerIn:parent; text:parent.surface ? parent.surface.instanceId : "Surface"; color:"white"; font.pixelSize:24 }
}
