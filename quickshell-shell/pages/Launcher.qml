import QtQuick
import "."
Rectangle {
    id:page
    property var manager
    property var surface
    color:"#202332"; radius:12
    focus:true
    Keys.onEscapePressed: surface.shown=false
    Column {
        anchors.fill:parent; anchors.margins:24; spacing:16
        Item {
            width:parent.width; height:30
            Text { anchors.left:parent.left; anchors.verticalCenter:parent.verticalCenter; text:"Applications"; color:"#eeeeff"; font.pixelSize:24 }
            Action { anchors.right:parent.right; text:"Close"; onClicked:page.surface.shown=false }
        }
        Rectangle {
            width:parent.width; height:40; color:"#303044"; radius:5
            TextInput { id:query; anchors.fill:parent; anchors.margins:10; color:"#eeeeff"; font.pixelSize:16; focus:true; Keys.onEscapePressed:page.surface.shown=false }
        }
        Repeater {
            model:['Terminal','Files','Browser','Settings'].filter(function(name) { return name.toLowerCase().indexOf(query.text.toLowerCase())>=0; })
            Action { required property string modelData; text:modelData; width:parent.width; onClicked:page.surface.shown=false }
        }
    }
    Connections { target:page.surface || null; function onShownChanged() { if(page.surface.shown) { query.text=""; query.forceActiveFocus(); } } }
}
