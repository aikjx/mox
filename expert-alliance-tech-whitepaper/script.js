// TOC active state tracking
const tocLinks = document.querySelectorAll('.toc-link');
const chapters = document.querySelectorAll('.chapter');

function updateActiveTOC() {
  let current = '';
  const scrollPos = window.scrollY + 120;
  
  chapters.forEach(chapter => {
    if (chapter.offsetTop <= scrollPos) {
      current = chapter.id;
    }
  });
  
  tocLinks.forEach(link => {
    link.classList.toggle('active', link.getAttribute('href') === '#' + current);
  });
}

window.addEventListener('scroll', updateActiveTOC, { passive: true });
updateActiveTOC();

// Smooth scroll for TOC links
tocLinks.forEach(link => {
  link.addEventListener('click', (e) => {
    e.preventDefault();
    const target = document.querySelector(link.getAttribute('href'));
    if (target) {
      target.scrollIntoView({ behavior: 'smooth' });
    }
  });
});
