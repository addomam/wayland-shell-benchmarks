import QtQuick
Rectangle {
    id: button
    property string text: ""
    property bool selected: false
    signal clicked()
    width: Math.max(64, label.implicitWidth + 24)
    height: 30
    radius: 5
    color: selected ? "#675ea8" : (mouse.containsMouse ? "#45435c" : "#303044")
    Text { id:label; anchors.centerIn:parent; text:button.text; color:"#eeeeff"; font.pixelSize:14 }
    MouseArea { id:mouse; anchors.fill:parent; hoverEnabled:true; onClicked:button.clicked() }
}
