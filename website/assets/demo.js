// The Wings demo: an AI chat window with the Wings sidebar beside it, playing the real flow on a loop.
// While the AI is busy the user types, saves to Wings, types again, saves again; the AI finishes;
// one click on a note pastes it back. Every concept page themes it through the CSS variables in demo.css.
(function () {
  const reduce = window.matchMedia('(prefers-reduced-motion: reduce)').matches;

  function build(root) {
    root.classList.add('wd');
    root.innerHTML = `
      <div class="wd-app" aria-hidden="true">
        <div class="wd-bar"><i></i><i></i><i></i><span>Claude</span></div>
        <div class="wd-chat">
          <div class="wd-msg wd-user">帮我把登录页重构一下，拆成小组件</div>
          <div class="wd-msg wd-ai"><span class="wd-status">正在工作中…</span><span class="wd-reply">好了，拆成了表单、按钮和错误提示三个组件。</span></div>
        </div>
        <div class="wd-input">
          <span class="wd-text"></span><span class="wd-caret"></span>
          <span class="wd-chip">保存到 Wings</span>
          <span class="wd-btn"><svg viewBox="0 0 16 16"><rect class="wd-stop" x="4.5" y="4.5" width="7" height="7" rx="1.5"/><path class="wd-send" d="M8 12.5V4M4.5 7.5 8 4l3.5 3.5"/></svg></span>
        </div>
      </div>
      <div class="wd-side" aria-hidden="true">
        <div class="wd-side-head"><img src="assets/wings-icon-1024.png" alt=""><span>Wings</span></div>
        <div class="wd-notes"></div>
        <div class="wd-toast">已粘贴，确认后再按回车</div>
      </div>
      <p class="wd-sr">演示：AI 正在回复时，把下一句存进 Wings；AI 回完后点一下便签，它回到输入框。</p>`;
  }

  const lines = ['顺便把单元测试也补上', '再写一段更新说明'];

  function run(root) {
    const $ = s => root.querySelector(s);
    const text = $('.wd-text'), chip = $('.wd-chip'), notes = $('.wd-notes'), ai = $('.wd-ai'), toast = $('.wd-toast'), input = $('.wd-input'), status = $('.wd-status');
    const wait = ms => new Promise(r => setTimeout(r, ms));

    async function type(s) {
      for (const ch of s) { text.textContent += ch; await wait(70 + Math.random() * 60); }
    }
    async function save(s) {
      chip.classList.add('on');
      await wait(650);
      chip.classList.add('press');
      await wait(220);
      chip.classList.remove('press', 'on');
      text.textContent = '';
      const n = document.createElement('div');
      n.className = 'wd-note';
      n.textContent = s;
      notes.appendChild(n);
      requestAnimationFrame(() => n.classList.add('in'));
      await wait(700);
    }

    async function loop() {
      for (;;) {
        notes.innerHTML = ''; text.textContent = ''; ai.classList.remove('done'); input.classList.remove('idle'); toast.classList.remove('on');
        input.classList.remove('filled'); status.textContent = '正在工作中…';
        await wait(900);
        for (const [i, s] of lines.entries()) { await type(s); await save(s); if (i === 0) status.textContent = '正在执行中…'; }
        await wait(500);
        ai.classList.add('done'); input.classList.add('idle');
        await wait(1300);
        const first = notes.firstElementChild;
        first.classList.add('aim');
        await wait(500);
        first.classList.add('press');
        await wait(200);
        first.classList.remove('press', 'aim');
        text.textContent = first.textContent;
        input.classList.add('filled');
        toast.classList.add('on');
        await wait(2600);
      }
    }

    if (reduce) {
      // A still frame of the story's middle: two notes waiting, the AI still answering.
      lines.forEach(s => { const n = document.createElement('div'); n.className = 'wd-note in'; n.textContent = s; notes.appendChild(n); });
      return;
    }
    // Start only once the demo is on screen.
    const io = new IntersectionObserver(es => { if (es.some(e => e.isIntersecting)) { io.disconnect(); loop(); } }, { threshold: 0.3 });
    io.observe(root);
  }

  document.querySelectorAll('[data-wings-demo]').forEach(root => { build(root); run(root); });
})();
