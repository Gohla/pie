document.addEventListener("DOMContentLoaded", (_) => {
  const codeClassPattern = /.*customdiff.*/;
  const diffLinePattern = /^(?<span><\/span>)?(?<diff>[-+])(?<rest>.*)/;
  const nonDiffLinePattern = /^ /;
  document.querySelectorAll("code").forEach((el) => {
    if (codeClassPattern.test(el.className)) {
      const lines = el.innerHTML.split("\n").map(line => {
        const match = line.match(diffLinePattern);
        if (!match) {
          return line.replace(nonDiffLinePattern, "");
        } else {
          const removal = match.groups.diff === "-";
          return `${match.groups.span ? `</span>` : ``}<span style="background-color: ${removal ? `#ffeef0` : `#f0fff4`}; display: inline-block; width: 100%;">${match.groups.rest}</span>`;
        }
      });
      el.innerHTML = lines.join("\n");
    }
  });
});
