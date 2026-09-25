import QtQuick
import "."
Rectangle {
    id:page
    property var manager
    property var surface
    color:"#202332"; radius:8
    Column {
        anchors.fill:parent; anchors.margins:20; spacing:16
        Text { text:"Separate popup surface"; color:"#eeeeff"; font.pixelSize:18 }
        Action { text:"Input region experiment"; onClicked:page.manager.toggle('input-test',page.surface.screen.name) }
        Action { text:"Close"; onClicked:page.surface.shown=false }
    }
}
