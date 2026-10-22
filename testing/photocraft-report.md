# photocraft: fredapps-approved-ci-green

Built 2026-10-22 01:17 UTC on mirror main `f4bc90b93f` (1 behind storytold main). Result `df51e9f873`.

- merged: **21**
- skipped by the skip list: **12**
- conflicts: **63** (52 with main itself, 11 only with PRs merged before them)
- not eligible (review, CI, draft...): {"no review by FredApps": 385, "latest review is changes_requested": 174, "approval is not on the current head": 82, "draft": 44, "bot author": 2, "latest review is commented": 137, "CI is none": 44, "CI is failure": 2, "CodeRabbit has 1 actionable findings on the current head": 25, "no successful CodeRabbit check": 129, "CodeRabbit has 5 actionable findings on the current head": 2, "CodeRabbit has 4 actionable findings on the current head": 1, "CodeRabbit has 2 actionable findings on the current head": 4, "CodeRabbit has 3 actionable findings on the current head": 2}

## Skipped

- #6383 Windows releases ship a symbols zip: the release build's PDBs for prof (Higgs32584): touches packaging/windows/package.ps1; touches packaging/windows/symbols.txt
- #5815 perf(doc): share the XMP packet, Duotone PSD data and unsupported adju (FinnNk): deny list: changes Adjustment::Unsupported.raw to Arc<[u8]> but is 340 commits behind main: main's crates/io/tests/adjust_warnings.rs:70 and #6815's psd_export.rs test still build it from a Vec, so it
- #7743 packaging: the portable zip holds the files, not a folder with the fil (Hamza0101): touches packaging/windows/package.ps1
- #7433 Type: add the Warp Text button to the options bar (Myosh1): deny list: its new test (warp_text_dialog_tests.rs:252) builds state::TextEdit without the `document` field that #6509 added; dropped by the owner's choice, #6509 stays
- #7659 Transform a type layer with a missing font as its pixels (#7278) (hectorviov): deny list: calls check_budget with 3 arguments after #7838 added a 4th (transform_cmds.rs:341); dropped by the owner's choice, #7838 stays
- #1646 feat: enable AVIF export in Save As, Export As and CLI (Streetblock): touches Cargo.lock; touches apps/photocraft-cli/Cargo.toml; touches apps/photocraft-web/Cargo.toml; touches apps/photocraft/Cargo.toml; touches crates/codecs/Cargo.toml; touches crates/io/Cargo.toml; 
- #6757 feat(io): added support for JPEG 2000 .jp2, .jpm, .jpf, .mj2, .j2c ima (peppepol): touches Cargo.lock; touches crates/codecs/Cargo.toml
- #5249 PSD: Distort smart filters and Invert open and save like Photoshop (abc2cb): added text a very long base64-like blob
- #4743 fix(ui): disable toolbar and tool options on the home screen (Ni-zav): deny list: disables the whole toolbar when no document is open; the tests #6907 (a_click_without_a_prior_hover_picks_the_tool) and #7595 (color_chips_stay_centred_when_the_tools_scroll) added later ru
- #3387 feat(codecs): add lossless JPEG XL export for 8/16-bit images (Ni-zav): touches Cargo.lock; touches crates/codecs/Cargo.toml
- #6510 Automation: resolve configured root aliases for absolute paths (#6426) (PerchAnton): touches .github/workflows/ci.yml
- #5677 Layers: saved option to collapse effects on document open (#5320) (bommerts): binary file: docs/images/layer-effects-on-open-after.png; binary file: docs/images/layer-effects-on-open-before.png

## Conflicts

- #6794 feat(ui): System Info shows the graphics card's own memory: duplicate key 'cameraRaw' in crates/ui-egui/src/i18n/cs.tsv
- #5082 fix(mixer-brush): Alt-click samples the foreground colour like the Bru: crates/engine/src/brush_cmds.rs, crates/paint/src/mixer.rs, crates/ui-egui/src/canvas.rs
- #7681 fix(actions): preserve scalar Selective Color settings in ATN export: crates/engine/src/atn_cmds.rs
- #7591 fix(atn): refuse strict export of canvas-bound selection modifies and : crates/engine/src/atn_cmds.rs, crates/engine/src/atn_cmds/tests.rs
- #7590 fix(atn): keep a Transform Selection's interpolation; refuse what the : crates/engine/src/atn_cmds/tests.rs
- #7588 fix(atn): refuse strict export of options the descriptors can't carry: crates/engine/src/atn_cmds.rs, crates/engine/src/atn_cmds/tests.rs
- #7396 Auto-Blend Layers: Seamless Tones and Colors turns on gain compensatio: crates/engine/src/align_cmds.rs
- #7281 perf(retouch): releasing a Blur/Smudge/Dodge stroke takes its live pix: crates/engine/src/retouch_cmds.rs
- #7255 fix(engine): guard trim and duplicate merged against documents exceedi: crates/engine/src/image_cmds.rs
- #5462 docs: keep progress status lines to the latest change: docs/brush-parity.md, docs/gaps.md, docs/target-app-parity.md, docs/ui-parity.md
- #7593 Refuse .atn export of crops with output size or resolution: crates/engine/src/atn_cmds/tests.rs
- #7585 Refuse strict .atn export of a mask/channel-targeted Free Transform: crates/engine/src/atn_cmds/tests.rs
- #7571 Refuse strict .atn export of Apply Image with a blending mask: crates/engine/src/atn_cmds/tests.rs
- #7611 Refuse strict ATN export of Select and Mask with brush strokes: crates/engine/src/atn_cmds.rs, crates/engine/src/atn_cmds/tests.rs
- #4783 Fix/4758 type extreme finite geometry 20261013s1215: crates/io/tests/psd_type_bounds.rs, crates/text/src/psd.rs
- #4883 feat(ui): show selection and active-layer bounds in the status bar: duplicate key 'cameraRaw' in crates/ui-egui/src/i18n/cs.tsv
- #3921 feat(layer-style): expose Bevel & Emboss highlight and shadow shading: crates/engine/src/layer_style.rs, crates/ui-egui/src/layer_style.rs
- #3193 fix(compose): resolve Dissolve coverage across adjustment, effects, a…: crates/compose/src/tests.rs, crates/gpu/tests/parity.rs
- #7624 Fade the alpha channel or Quick Mask an edit changed (#7382): crates/engine/src/edit_menu_cmds.rs
- #7605 Lasso tools: the outline is solid until the selection is made (#7495): crates/ui-egui/src/canvas.rs
- #7677 Export As: show the size the file will have (#6595): crates/ui-egui/src/export_dialog.rs
- #7637 Record one action call when playback pauses for a step dialog: duplicate key 'cameraRaw' in crates/ui-egui/src/i18n/cs.tsv
- #7574 Move tool: moving selected pixels shrinks the selection, transform con: crates/engine/src/float_cmds.rs, crates/engine/src/float_cmds/tests.rs, docs/ui-parity.md
- #7968 fix(atn): a Group Layers step keeps its explicit layer through .atn (#: crates/engine/src/atn_cmds/tests.rs
- #7848 fix(retouch): preserve Smudge pickup with live-pixel stroke commits: crates/engine/src/retouch_cmds.rs
- #7800 Guides: show the X or Y position beside the pointer while dragging : docs/ui-parity.md
- #7759 Actions: a setting Photoshop's event cannot hold refuses the step: crates/engine/src/atn_cmds.rs, crates/engine/src/atn_cmds/tests.rs
- #7742 Actions: a fractional setting an integer field cannot hold is refused,: crates/engine/src/atn_cmds.rs, crates/engine/src/atn_cmds/tests.rs
- #7656 Refuse .atn export of resolution-only Image Size without keepPrintSize: crates/engine/src/atn_cmds/tests.rs
- #7662 Preferences: Zoom Resizes Windows resizes floating document windows, a: crates/engine/src/prefs.rs, crates/ui-egui/src/prefs_ui.rs, docs/scorecard.md
- #7232 fix(atn): export recorded layer.select with name parameter (#7185): crates/engine/src/atn_cmds/tests.rs
- #7179 fix(ui): explain Content-Aware Scale restrictions (#7107): docs/gaps.md, docs/ui-parity.md
- #7115 fix(atn): map Apply Layer Comp steps by name (#7101): docs/file-format-parity.md, docs/gaps.md
- #6897 GPU fallback: Retry GPU after an error rebuilds the GPU canvas now, no: crates/ui-egui/src/lib.rs, docs/control-protocol.md
- #6453 Type options: drag the "T" icon to change the font size, as in Photosh: docs/ui-parity.md
- #6468 fix(photocraft): `--control 0` logs the port actually bound (#6427): apps/photocraft/src/control_server.rs
- #7262 fix(atn): reject numeric layer references in .atn export (#7248): crates/engine/src/jobs.rs
- #7249 fix(atn): reject non-replace select.rect modes in .atn export (#7194): crates/engine/src/atn_cmds/tests.rs
- #5988 Raw: a Nikon maker note's ColorSpace sets the preview colour space (#5: docs/gaps.md
- #5980 Raw: the Nikon D4 white level follows the sample depth (#5978): docs/file-format-parity.md, docs/gaps.md
- #7638 Map the bevel's shading through its gloss contour, not its light (#736: crates/compose/src/effects.rs
- #7816 docs(scorecard): regenerate after Enable Gestures (169 settings): docs/scorecard.md
- #7721 fix(actions): refuse lossy fractional Crystallize ATN export: crates/engine/src/atn_cmds.rs
- #7857 Replace Contents: contents of another size keep the placement's centre: crates/engine/src/smart_cmds/tests.rs
- #7258 fix(atn): reject off-center radial blur in .atn export (#7234): crates/engine/src/atn_cmds.rs, crates/engine/src/atn_cmds/tests.rs
- #7795 Print: Scale to Fit Media fills the driver's printable area (#7782): crates/engine/src/print_cmds/tests.rs
- #5470 feat(tiff): a tiffCompression option for scripted TIFF saves: crates/engine/src/file_cmds/tests.rs, docs/file-format-parity.md
- #5421 fix(format): serialise directory bundle saves with a bundle-wide lock: crates/format/src/store.rs
- #5420 fix(format): report a failed directory flush after a save instead of i: crates/format/src/store.rs
- #5447 Isolate live distortion tools per document (#5115): crates/ui-egui/src/lib.rs
- #5443 Actions panel menu: Load Actions and the rest of the commands (#5021): duplicate key 'cameraRaw' in crates/ui-egui/src/i18n/cs.tsv
- #5816 Auto-Blend Layers: blend a layer the canvas crops to all-overlap (#574: crates/engine/src/align_cmds.rs
- #8114 Layers panel: a click on the empty space below the layers deselects th: crates/ui-egui/src/layer_shortcut_tests.rs
- #8097 i18n(pt-br): translate 131 runtime labels that still showed in English: duplicate key 'cameraRaw' in crates/ui-egui/src/i18n/pt-br.tsv
- #7395 Auto-Blend Layers: mask and blend the whole layers, past the canvas to: crates/engine/src/align_cmds.rs
- #7161 i18n(zh-hans): translate the 14 counted status messages missing from S: duplicate key 'cameraRaw' in crates/ui-egui/src/i18n/zh-hans.tsv
- #7160 i18n(ko): translate the 14 counted status messages missing from Korean: duplicate key 'cameraRaw' in crates/ui-egui/src/i18n/ko.tsv
- #7159 i18n(id): translate the 14 counted status messages missing from Indone: duplicate key 'cameraRaw' in crates/ui-egui/src/i18n/id.tsv
- #7150 i18n(pl): translate the 14 counted status messages missing from Polish: duplicate key 'fontWeight' in crates/ui-egui/src/i18n/pl.tsv
- #6918 fix(shortcuts): Page Up/Down and arrow-key shortcuts never fired (#682: crates/engine/src/prefs.rs, crates/engine/src/prefs/tests.rs, crates/ui-egui/src/shortcuts.rs
- #3868 feat(ui): nest light and dark themes into submenus: duplicate key 'cameraRaw' in crates/ui-egui/src/i18n/cs.tsv
- #3532 Move tool: show all six Align icons and an icon menu for Distribute (#: ATTRIBUTION.md
- #8324 fix(actions): Hue/Saturation uses Photoshop's hue key H in .atn files : crates/engine/src/atn_cmds/tests.rs

## Merged (oldest approval first; flags are for a closer look)

- #6415 Save for Web: Width, Height, Percent and Resample in Image Size, linke (marcoarnulfo)
- #6815 PSD: keep Photoshop 2026 Light adjustment layers verbatim and warn (#6 (Higgs32584)
- #6492 Fail control startup promptly when its port is occupied (denstu473) [flags: Command::new, TcpStream]
- #5169 fix(ui): show Paint Bucket first in the shared toolbar slot (Ni-zav)
- #3914 fix(automation): release idle headless TCP server ports (Ni-zav) [flags: TcpStream]
- #6025 Rectangular Marquee: Style: Fixed Size selects exactly that many pixel (wstock)
- #7058 Crop: X with a ratio preset turns the frame instead of shrinking it (# (davidmann4)
- #7622 Show every blend mode in the open dropdown list (#7261) (hectorviov)
- #7305 Codecs: read Photoshop Raw (.raw) with retained header (nrq6993-a11y)
- #8039 Compose: Extend Softer outer glows past Size (#7899) (Sir-Starch)
- #8035 CLI: reject ignored or conflicting mcp/serve launch options (wanikhawar)
- #7994 Fix Paint Bucket target with Gradient-style crosshair (envy-ai)
- #6417 Context menus: an overflowing menu shows ▲/▼ scroll arrows (#2091) (Higgs32584)
- #5865 Save: a flat file that keeps the document is saved in place (#5733) (Higgs32584)
- #3356 CMYK documents keep their exact inks: export, PSD merged image and Fla (SBaksa)
- #1991 feat(actions): import and export reusable Actions JSON files (Ni-zav)
- #8250 fix(io): read a placed layer's custom warp over its source rect (#8227 (Korg1ch)
- #6401 PSD smart filters: Sharpen, Dust & Scratches and Hue/Saturation stay e (Higgs32584)
- #8237 Clipboard: a panic while reading the clipboard is reported, not a cras (Higgs32584)
- #8232 Plug-ins: a reduced live preview scales the settings a manifest marks  (Higgs32584)
- #6393 JPEG: 12-bit and arithmetic-coded files open (#4934, #5003) (Higgs32584) [flags: std::env::var]
