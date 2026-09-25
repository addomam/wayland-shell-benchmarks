const byId = id => document.getElementById(id);
byId('launcher-button').addEventListener('click', () => shell.surfaces.toggle('launcher'));
byId('menu-button').addEventListener('click', () => shell.surfaces.toggle('menu'));
shell.workspaces.subscribe(state => {
  const root = byId('workspaces');
  root.replaceChildren(...state.list.map(workspace => {
    const button = document.createElement('button');
    button.textContent = workspace.name;
    button.className = workspace.id === state.current ? 'active' : '';
    button.addEventListener('click', () => shell.workspaces.activate(workspace.id));
    return button;
  }));
  document.title = `workspaces:${state.list.map(w => w.id).join(',')} active:${state.current}`;
});
function tick() { byId('clock').textContent = new Date().toLocaleTimeString(); }
tick(); setInterval(tick, 1000);
