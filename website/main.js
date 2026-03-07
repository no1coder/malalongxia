// ============================================================
// MalaLongxia Website - Interactions & Animations
// ============================================================

(function () {
  'use strict';

  // ----- Scroll-triggered animations -----
  function initScrollAnimations() {
    var elements = document.querySelectorAll('.animate-on-scroll');
    if (!elements.length) return;

    var observer = new IntersectionObserver(
      function (entries) {
        entries.forEach(function (entry) {
          if (entry.isIntersecting) {
            entry.target.classList.add('visible');
            observer.unobserve(entry.target);
          }
        });
      },
      { threshold: 0.1, rootMargin: '0px 0px -40px 0px' }
    );

    elements.forEach(function (el, i) {
      el.style.transitionDelay = (i % 6 * 80) + 'ms';
      observer.observe(el);
    });
  }

  // ----- Navbar scroll effect -----
  function initNavbar() {
    var nav = document.getElementById('nav');
    if (!nav) return;

    window.addEventListener('scroll', function () {
      if (window.scrollY > 20) {
        nav.classList.add('scrolled');
      } else {
        nav.classList.remove('scrolled');
      }
    }, { passive: true });
  }

  // ----- Mobile menu toggle -----
  function initMobileMenu() {
    var btn = document.getElementById('navMenuBtn');
    var mobile = document.getElementById('navMobile');
    if (!btn || !mobile) return;

    btn.addEventListener('click', function () {
      mobile.classList.toggle('open');
    });

    mobile.querySelectorAll('a').forEach(function (link) {
      link.addEventListener('click', function () {
        mobile.classList.remove('open');
      });
    });
  }

  // ----- Smooth scroll for anchor links -----
  function initSmoothScroll() {
    document.querySelectorAll('a[href^="#"]').forEach(function (link) {
      link.addEventListener('click', function (e) {
        var targetId = link.getAttribute('href');
        if (targetId === '#') return;
        var target = document.querySelector(targetId);
        if (!target) return;
        e.preventDefault();
        var navHeight = 64;
        var y = target.getBoundingClientRect().top + window.scrollY - navHeight;
        window.scrollTo({ top: y, behavior: 'smooth' });
      });
    });
  }

  // ----- macOS download modal -----
  function initMacModal() {
    var btn = document.getElementById('macDownloadBtn');
    var modal = document.getElementById('macModal');
    var closeBtn = document.getElementById('macModalClose');
    if (!btn || !modal) return;

    btn.addEventListener('click', function () {
      modal.classList.add('open');
    });

    if (closeBtn) {
      closeBtn.addEventListener('click', function () {
        modal.classList.remove('open');
      });
    }

    modal.addEventListener('click', function (e) {
      if (e.target === modal) {
        modal.classList.remove('open');
      }
    });
  }

  // ----- Download then redirect to tips -----
  function initDownloadRedirect() {
    document.querySelectorAll('[data-download]').forEach(function (link) {
      link.addEventListener('click', function () {
        setTimeout(function () {
          window.location.href = 'tips.html';
        }, 1500);
      });
    });
  }

  // ----- Initialize -----
  document.addEventListener('DOMContentLoaded', function () {
    initNavbar();
    initMobileMenu();
    initSmoothScroll();
    initScrollAnimations();
    initMacModal();
    initDownloadRedirect();
  });
})();
