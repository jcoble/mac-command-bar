/*
 * Adapted from code-yeongyu/web-terminal src/client/touch-scroll.ts
 * (MIT License, Copyright (c) 2026 YeonGyu Kim - keep this notice).
 * Changes: plain JavaScript; long-press selection and mouse-tracking handoff left out.
 *
 * Single-finger drag scrolling. The terminal has `touch-action: none`, so drags are
 * turned into pixel WheelEvents, which ghostty's own wheel handler consumes (including
 * alt-screen arrow keys). Taps reach the browser untouched so it focuses the WebView and shows the keyboard.
 */
function attachTouchScroll(container, onTap) {
  const DRAG_SLOP_PX = 8;
  const SCROLL_MULTIPLIER = 2;
  let lastY;
  let dragging = false;

  const onTouchStart = (event) => {
    lastY = event.touches.length === 1 ? event.touches[0].clientY : undefined;
    dragging = false;
  };
  const onTouchMove = (event) => {
    const touch = event.touches[0];
    if (touch === undefined || lastY === undefined) return;
    const deltaY = lastY - touch.clientY;
    if (!dragging && Math.abs(deltaY) < DRAG_SLOP_PX) return;
    dragging = true;
    lastY = touch.clientY;
    event.preventDefault();
    container.dispatchEvent(new WheelEvent('wheel', {
      deltaY: deltaY * SCROLL_MULTIPLIER, deltaMode: WheelEvent.DOM_DELTA_PIXEL, bubbles: true, cancelable: true,
    }));
  };
  const onTouchEnd = (event) => {
    // Keep every touchend from ghostty's canvas handler: it calls preventDefault, which cancels
    // the browser's tap, and only that tap gives the WebView Android focus and the keyboard.
    // A drag must not focus at all, or a scroll flick would summon the keyboard.
    event.stopPropagation();
    if (!dragging && lastY !== undefined) onTap();
    lastY = undefined;
    dragging = false;
  };
  // Capture phase: runs before ghostty's canvas handlers so a drag can stop the event reaching them.
  container.addEventListener('touchstart', onTouchStart, { capture: true, passive: true });
  container.addEventListener('touchmove', onTouchMove, { capture: true, passive: false });
  container.addEventListener('touchend', onTouchEnd, { capture: true, passive: false });
  container.addEventListener('touchcancel', onTouchEnd, { capture: true, passive: false });
}
