document.addEventListener("DOMContentLoaded", (_) => {
  const codeClassPattern = /.*customdiff.*/;
  const diffLinePattern = /^(?<span><\/span>)?(?<diff>[-+])(?<rest>.*)/;
  document.querySelectorAll("code").forEach((el) => {
    if (codeClassPattern.test(el.className)) {
      const lines = el.innerHTML.split("\n").map(line => {
        const match = line.match(diffLinePattern);
        if (!match) {
          return line;
        } else {
          const removal = match.groups.diff === "-";
          return `${match.groups.span ? `</span>` : ``}<span style="background-color: ${removal ? `#ffeef0` : `#f0fff4`};">${match.groups.rest}</span>`;
        }
      });
      el.innerHTML = lines.join("\n");
    }
  });
});
