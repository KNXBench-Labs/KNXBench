/* Progressive enhancement only: links, disclosures and video controls work without JS. */
'use strict';
const videos = [...document.querySelectorAll('video')];
for (const video of videos) {
  video.addEventListener('play', () => {
    for (const other of videos) if (other !== video) other.pause();
  });
}
document.addEventListener('visibilitychange', () => {
  if (document.hidden) for (const video of videos) video.pause();
});
const menu = document.querySelector('.mobile-menu');
if (menu) {
  menu.addEventListener('click', event => {
    if (event.target.closest('a')) menu.open = false;
  });
  menu.addEventListener('keydown', event => {
    if (event.key === 'Escape' && menu.open) {
      menu.open = false;
      menu.querySelector('summary').focus();
    }
  });
}
