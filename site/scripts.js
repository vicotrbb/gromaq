const copyButtons = document.querySelectorAll("[data-copy-target]");

copyButtons.forEach((button) => {
  const originalText = button.textContent.trim();

  button.addEventListener("click", async () => {
    const targetId = button.getAttribute("data-copy-target");
    const target = targetId ? document.getElementById(targetId) : null;
    if (!target) return;

    try {
      await navigator.clipboard.writeText(target.textContent.trim());
      button.textContent = "Copied";
      window.setTimeout(() => {
        button.textContent = originalText;
      }, 1600);
    } catch {
      button.textContent = "Select command";
      window.setTimeout(() => {
        button.textContent = originalText;
      }, 2200);
    }
  });
});
