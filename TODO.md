# TODO

## Check by hand

- [x] Ctrl+C copies selected message text to the clipboard, and copies text selected in the edit box.
- [x] The character counter changes colour as it nears the limit, and text past 2000 characters is cut off in the composer and the edit box.

## Message text selection

- [x] Clicking outside a message clears the selection (only Escape does now).
- [ ] A selection can span more than one message.
- [x] Ctrl+C in the composer copies the composer's own selection, not a selection in a message.
- [ ] Dragging through the gap between blocks (paragraphs, code blocks, lists) keeps extending the selection.

## LaTeX

- [ ] Matrices (`\begin{matrix}`, `pmatrix`, `bmatrix`).
- [ ] Root index for `\sqrt[n]{x}` (the index is dropped now).
- [ ] `\left` / `\right` brackets that grow with their contents.
- [ ] A better fallback for superscripts and subscripts that have no Unicode form (now shown as `^(…)`).

## Code highlighting

- [ ] More languages beyond Rust, JS/TS, Python, C-style, shell and JSON (for example HTML, CSS, SQL, YAML, TOML).
