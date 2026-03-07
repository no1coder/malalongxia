// ============================================================
// MalaLongxia Website - Interactions & Animations
// ============================================================

(function () {
  'use strict';

  // ----- Scroll-triggered animations -----
  function initScrollAnimations() {
    const elements = document.querySelectorAll('.animate-on-scroll');
    if (!elements.length) return;

    const observer = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          if (entry.isIntersecting) {
            entry.target.classList.add('visible');
            observer.unobserve(entry.target);
          }
        });
      },
      { threshold: 0.1, rootMargin: '0px 0px -40px 0px' }
    );

    elements.forEach((el, i) => {
      el.style.transitionDelay = `${i % 6 * 80}ms`;
      observer.observe(el);
    });
  }

  // ----- Navbar scroll effect -----
  function initNavbar() {
    const nav = document.getElementById('nav');
    if (!nav) return;

    let lastScroll = 0;
    window.addEventListener('scroll', () => {
      const scrollY = window.scrollY;
      if (scrollY > 20) {
        nav.classList.add('scrolled');
      } else {
        nav.classList.remove('scrolled');
      }
      lastScroll = scrollY;
    }, { passive: true });
  }

  // ----- Mobile menu toggle -----
  function initMobileMenu() {
    const btn = document.getElementById('navMenuBtn');
    const mobile = document.getElementById('navMobile');
    if (!btn || !mobile) return;

    btn.addEventListener('click', () => {
      mobile.classList.toggle('open');
    });

    // Close menu when clicking a link
    mobile.querySelectorAll('a').forEach((link) => {
      link.addEventListener('click', () => {
        mobile.classList.remove('open');
      });
    });
  }

  // ----- Smooth scroll for anchor links -----
  function initSmoothScroll() {
    document.querySelectorAll('a[href^="#"]').forEach((link) => {
      link.addEventListener('click', (e) => {
        const targetId = link.getAttribute('href');
        if (targetId === '#') return;
        const target = document.querySelector(targetId);
        if (!target) return;
        e.preventDefault();
        const navHeight = 64;
        const y = target.getBoundingClientRect().top + window.scrollY - navHeight;
        window.scrollTo({ top: y, behavior: 'smooth' });
      });
    });
  }

  // ----- Download then redirect to tips -----
  function initDownloadRedirect() {
    document.querySelectorAll('[data-download]').forEach((link) => {
      link.addEventListener('click', (e) => {
        // Let the browser start the download via default href behavior,
        // then redirect to tips page after a short delay
        setTimeout(() => {
          window.location.href = 'tips.html';
        }, 1500);
      });
    });
  }

  // ----- Initialize -----
  document.addEventListener('DOMContentLoaded', () => {
    initNavbar();
    initMobileMenu();
    initSmoothScroll();
    initScrollAnimations();
    initDownloadRedirect();
  });
})();
