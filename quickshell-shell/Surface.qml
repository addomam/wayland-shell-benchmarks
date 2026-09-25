import QtQuick
import Quickshell
import Quickshell.Wayland

PanelWindow {
    id: surface
    required property var manager
    required property var spec
    required property string instanceId
    property bool shown: true
    visible: shown
    color: "transparent"
    implicitWidth: spec.width
    implicitHeight: spec.height
    anchors.top: spec.anchors.indexOf('top') >= 0
    anchors.bottom: spec.anchors.indexOf('bottom') >= 0
    anchors.left: spec.anchors.indexOf('left') >= 0
    anchors.right: spec.anchors.indexOf('right') >= 0
    margins.top: spec.margins.top || 0
    margins.bottom: spec.margins.bottom || 0
    margins.left: spec.margins.left || 0
    margins.right: spec.margins.right || 0
    exclusiveZone: spec.exclusiveZone
    WlrLayershell.namespace: "qs-shell-" + instanceId
    WlrLayershell.layer: ({background:WlrLayer.Background,bottom:WlrLayer.Bottom,top:WlrLayer.Top,overlay:WlrLayer.Overlay})[spec.layer]
    WlrLayershell.keyboardFocus: ({none:WlrKeyboardFocus.None,exclusive:WlrKeyboardFocus.Exclusive,'on-demand':WlrKeyboardFocus.OnDemand})[spec.keyboardInteractivity]
    mask: Region { item: content.item && content.item.inputRegion ? content.item.inputRegion : content }
    SurfaceContent { id:content; anchors.fill:parent; surface:surface }
    onVisibleChanged: if(visible) console.log("READY " + instanceId)
}
