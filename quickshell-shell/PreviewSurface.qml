import QtQuick
import Quickshell

FloatingWindow {
    id: surface
    required property var manager
    required property var spec
    required property string instanceId
    property bool shown: true
    visible: shown
    title: "qs-shell-" + instanceId
    color: "transparent"
    implicitWidth: spec.width || 1000
    implicitHeight: spec.height || 500
    SurfaceContent { anchors.fill:parent; surface:surface }
    onVisibleChanged: if(visible) console.log("READY " + instanceId)
}
