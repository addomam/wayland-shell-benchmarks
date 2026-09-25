import QtQuick

Loader {
    id: content
    required property var surface
    source: Qt.resolvedUrl(surface.spec.page)
    onLoaded: {
        item.manager = surface.manager;
        item.surface = surface;
        console.log("PAGE " + surface.instanceId + " " + surface.spec.page);
    }
    onStatusChanged: if(status===Loader.Error) console.error("PAGE_ERROR " + surface.instanceId)
}
