/* Language preference: honor an explicit choice, otherwise negotiate from the
 * browser's preference list on the English pages only. Never redirects away
 * from a /zh URL someone opened on purpose. The site works without this file —
 * the header switcher is a plain link. */
(function () {
  var here = location.pathname;
  var onZh = here === "/zh" || here.indexOf("/zh/") === 0;
  try {
    var q = new URLSearchParams(location.search).get("lang");
    if (q === "en" || q === "zh") {
      localStorage.setItem("lang", q);
      history.replaceState(null, "", here + location.hash);
    }
    if (!onZh) {
      var pref = localStorage.getItem("lang");
      if (pref === null) {
        /* First supported tag wins. Simplified variants only — Traditional
         * readers keep English rather than being forced into 简体. */
        var langs = navigator.languages || [navigator.language || ""];
        for (var i = 0; i < langs.length; i++) {
          var t = (langs[i] || "").toLowerCase();
          if (t === "zh" || /^zh-(hans|cn|sg|my)/.test(t)) { pref = "zh"; break; }
          if (/^(en|zh)(-|$)/.test(t)) break;
        }
      }
      if (pref === "zh") {
        var to = { "/": "/zh", "/docs": "/zh/docs" }[here];
        if (to) location.replace(to + location.hash);
      }
    }
  } catch (e) {}
})();
