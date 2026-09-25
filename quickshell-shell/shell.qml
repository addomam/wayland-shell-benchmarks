import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Hyprland
import "config.js" as Config

ShellRoot {
    id: runtime
    property var instances: ({})
    property var desired: []
    property bool initialized: false
    property bool preview: Quickshell.env("SHELL_PREVIEW") === "1"
    property string lastConfig: ""
    property string lastControl: ""
    property string currentWorkspace: Hyprland.focusedWorkspace ? String(Hyprland.focusedWorkspace.id) : ""
    property var workspaces: Hyprland.workspaces.values
    onCurrentWorkspaceChanged: console.log("WORKSPACES current=" + currentWorkspace)
    function activate(id) {
        if (!Number.isInteger(id) || id < 1 || id > 100) return;
        Hyprland.dispatch('hl.dsp.focus({ workspace = "' + id + '" })');
    }
    function toggle(id, output) {
        for (const key of Object.keys(instances).sort()) {
            const w = instances[key];
            if ((w.spec.id === id || key === id) && (!output || w.screen.name === output)) {
                w.shown = !w.shown;
                console.log("TOGGLE " + key + " " + (w.shown ? "shown" : "hidden"));
                if (!output) return;
            }
        }
    }
    function reconcile(specs) {
        const expanded = Config.expand(specs, Quickshell.screens);
        const next = {};
        for (const entry of expanded) {
            let w = instances[entry.key];
            if (!w) {
                w = (preview ? previewFactory : layerFactory).createObject(runtime, {manager:runtime,spec:entry.spec,screen:entry.screen,instanceId:entry.key,shown:entry.spec.visible});
                console.log("CREATE " + entry.key);
            } else {
                if (w.spec.visible !== entry.spec.visible) w.shown = entry.spec.visible;
                w.spec = entry.spec;
            }
            next[entry.key] = w;
        }
        for (const key in instances) if (!next[key]) {
            instances[key].shown = false;
            instances[key].destroy();
            console.log("DESTROY " + key);
        }
        instances = next;
        desired = specs;
    }
    function applyConfig() {
        const text = config.text();
        if (text === lastConfig) return;
        lastConfig = text;
        try {
            const specs = Config.parse(text);
            for (const s of specs) {
                const component = Qt.createComponent(Qt.resolvedUrl(s.page));
                if (component.status !== Component.Ready) throw Error(component.errorString());
            }
            reconcile(specs);
            console.log("CONFIG applied " + specs.length);
        } catch (e) { console.error("CONFIG_ERROR " + e); }
    }
    FileView {
        id: config
        path: Quickshell.env("SHELL_CONFIG") || Quickshell.shellPath("surfaces.json")
        blockLoading: true
        watchChanges: true
        onFileChanged: configDebounce.restart()
        onLoaded: if (runtime.initialized) runtime.applyConfig()
    }
    Timer { id: configDebounce; interval: 300; onTriggered: config.reload() }
    FileView { id: control; path: Quickshell.env("SHELL_CONTROL") || "/tmp/quickshell-shell.control"; printErrors:false; blockWrites:true }
    Timer {
        interval:150; repeat:true; running:true
        onTriggered: control.reload()
    }
    Connections {
        target: control
        function onLoaded() {
            const text = control.text();
            if (!text || text === runtime.lastControl) return;
            runtime.lastControl = text;
            control.setText("");
            for (const line of text.trim().split(/\n/)) {
                const args = line.trim().split(/\s+/);
                if (args[0] === 'toggle') runtime.toggle(args[1]);
                else if (args[0] === 'workspace') runtime.activate(Number(args[1]));
                else if (args[0] === 'reload') { config.reload(); Quickshell.reload(false); }
                else if (args[0] === 'quit') Qt.quit();
            }
        }
    }
    Connections { target:Quickshell; function onScreensChanged() { if(runtime.initialized) runtime.reconcile(runtime.desired); } }
    Variants {
        model: Array.from(new Set(runtime.desired.map(s => s.page).concat(['pages/Action.qml'])))
        FileView {
            required property string modelData
            path: Quickshell.shellPath(modelData)
            watchChanges: true
            onFileChanged: frontendDebounce.restart()
        }
    }
    Timer { id:frontendDebounce; interval:300; onTriggered:Quickshell.reload(false) }
    Component { id:layerFactory; Surface {} }
    Component { id:previewFactory; PreviewSurface {} }
    Component.onCompleted: { initialized = true; applyConfig(); }
}
