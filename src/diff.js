document.addEventListener("DOMContentLoaded", (_) => {
  const codeClassPattern = /.*customdiff.*/;
  const diffLinePattern = /^(?<span><\/span>)?(?<kind>[-+]|@@)(?<rest>.*)/;
  const nonDiffLinePattern = /^ /;
  document.querySelectorAll("code").forEach((el) => {
    if(codeClassPattern.test(el.className)) {
      const lines = el.innerHTML
        .split("\n")
        .map(line => {
          const match = line.match(diffLinePattern);
          if(!match) {
            return line.replace(nonDiffLinePattern, "");
          } else {
            const kind = match.groups.kind;
            if(kind === "@@") {
              return null;
            }
            const removal = kind === "-";
            return `${match.groups.span ? `</span>` : ``}<span style="background-color: ${removal ? `#ffeef0` : `#f0fff4`}; display: inline-block; width: 100%;">${match.groups.rest}</span>`;
          }
        })
        .filter(line => line != null)
      ;
      el.innerHTML = lines.join("\n");
    }
  });
});
