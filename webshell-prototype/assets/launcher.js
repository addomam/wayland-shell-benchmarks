const input = document.querySelector('#search');
input.addEventListener('input', () => {
  for (const item of document.querySelectorAll('main div'))
    item.hidden = !item.textContent.toLowerCase().includes(input.value.toLowerCase());
});
document.getElementById('close').addEventListener('click', () => shell.surfaces.toggle('launcher'));
document.addEventListener('keydown', e => {
  if (e.key === 'Escape') shell.surfaces.toggle('launcher');
});
