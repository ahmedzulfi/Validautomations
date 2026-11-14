# glowkit catalog

Which components from popular web UI libraries have a glowkit equivalent.
**Generated from the code** by `scripts/catalog.py` (every demo declares what it `covers`).

glowkit components are original Rust implementations of these UI patterns, not ports of their code.
Some items are hard or impossible in an immediate-mode GPU painter without images/3D/DOM
(e.g. video text, 3D model viewers, real backdrop blur); those are marked as open.

## Summary

| Library | Covered | % |
|---|---|---|
| [React Bits](https://reactbits.dev) | 114 / 114 | 100% |
| [Magic UI](https://magicui.design) | 69 / 69 | 100% |
| [Aceternity UI](https://ui.aceternity.com) | 79 / 79 | 100% |
| [Motion Primitives](https://motion-primitives.com) | 33 / 33 | 100% |
| [shadcn/ui](https://ui.shadcn.com) | 58 / 58 | 100% |
| [Vercel AI Elements](https://ai-sdk.dev/elements) | 33 / 33 | 100% |
| [prompt-kit](https://www.prompt-kit.com) | 19 / 19 | 100% |
| [ElevenLabs UI](https://ui.elevenlabs.io) | 16 / 16 | 100% |
| [Evil Charts](https://evilcharts.com) | 6 / 6 | 100% |
| [MVP Blocks](https://blocks.mvp-subha.me) | 25 / 25 | 100% |
| [React Flow](https://reactflow.dev) | 12 / 12 | 100% |
| [Tremor](https://tremor.so) | 12 / 12 | 100% |
| **Total** | **476 / 476** | **100%** |

glowkit itself ships **457** gallery entries.

## React Bits

114 of 114 covered.

| Component | glowkit |
|---|---|
| Split Text | ✅ `split-text` |
| Blur Text | ✅ `blur-text` |
| Circular Text | ✅ `circular-text` |
| Text Type | ✅ `typewriter-text` |
| Shuffle | ✅ `shuffle-text` |
| Shiny Text | ✅ `shiny-text` |
| Text Pressure | ✅ `proximity-text` |
| Curved Loop | ✅ `curved-loop` |
| Fuzzy Text | ✅ `fuzzy-text` |
| Gradient Text | ✅ `gradient-text` |
| Falling Text | ✅ `falling-text` |
| Text Cursor | ✅ `text-cursor` |
| Decrypted Text | ✅ `decrypted-text` |
| True Focus | ✅ `true-focus` |
| Scroll Float | ✅ `scroll-float` |
| Scroll Reveal | ✅ `scroll-reveal-text` |
| ASCII Text | ✅ `ascii-text` |
| Scrambled Text | ✅ `scrambled-text` |
| Rotating Text | ✅ `rotating-text` |
| Glitch Text | ✅ `glitch-text` |
| Scroll Velocity | ✅ `scroll-velocity` |
| Variable Proximity | ✅ `proximity-text` |
| Count Up | ✅ `count-up` |
| Animated Content | ✅ `animated-content` |
| Fade Content | ✅ `fade-content` |
| Electric Border | ✅ `electric-border` |
| Pixel Transition | ✅ `pixel-transition` |
| Glare Hover | ✅ `glare-card` |
| Logo Loop | ✅ `marquee` |
| Target Cursor | ✅ `target-cursor` |
| Magnet Lines | ✅ `magnet-lines` |
| Click Spark | ✅ `click-spark` |
| Magnet | ✅ `magnet` |
| Sticker Peel | ✅ `sticker-peel` |
| Pixel Trail | ✅ `pixel-trail` |
| Cubes | ✅ `tilt-cubes` |
| Metallic Paint | ✅ `metallic-paint` |
| Noise | ✅ `grain-noise` |
| Shape Blur | ✅ `shape-blur` |
| Crosshair | ✅ `crosshair` |
| Image Trail | ✅ `card-trail` |
| Ribbons | ✅ `ribbon-trail` |
| Splash Cursor | ✅ `pointer-trail` |
| Meta Balls | ✅ `meta-balls` |
| Blob Cursor | ✅ `blob-cursor`, `spring-chain` |
| Star Border | ✅ `star-border-button` |
| Laser Flow | ✅ `laser-flow` |
| Gradual Blur | ✅ `gradual-blur` |
| Animated List | ✅ `animated-list` |
| Scroll Stack | ✅ `scroll-stack` |
| Bubble Menu | ✅ `bubble-menu` |
| Magic Bento | ✅ `magic-bento` |
| Circular Gallery | ✅ `card-carousel-3d`, `ring-carousel` |
| Card Nav | ✅ `card-nav` |
| Stack | ✅ `stack-cards` |
| Fluid Glass | ✅ `fluid-glass` |
| Pill Nav | ✅ `pill-nav` |
| Tilted Card | ✅ `flip-card-3d`, `tilt-card` |
| Masonry | ✅ `masonry` |
| Glass Surface | ✅ `fluid-glass`, `glass-surface` |
| Dome Gallery | ✅ `card-carousel-3d` |
| Chroma Grid | ✅ `chroma-grid` |
| Folder | ✅ `folder` |
| Staggered Menu | ✅ `staggered-menu` |
| Model Viewer | ✅ `model-viewer` |
| Lanyard | ✅ `lanyard` |
| Profile Card | ✅ `profile-card` |
| Dock | ✅ `dock` |
| Gooey Nav | ✅ `gooey-nav` |
| Pixel Card | ✅ `pixel-card` |
| Carousel | ✅ `carousel` |
| Spotlight Card | ✅ `spotlight-card` |
| Flying Posters | ✅ `flying-posters` |
| Card Swap | ✅ `card-swap` |
| Glass Icons | ✅ `glass-icons` |
| Decay Card | ✅ `decay-card` |
| Flowing Menu | ✅ `flowing-menu` |
| Elastic Slider | ✅ `elastic-slider` |
| Counter | ✅ `sliding-number` |
| Infinite Menu | ✅ `infinite-menu` |
| Stepper | ✅ `animated-stepper` |
| Bounce Cards | ✅ `bounce-cards` |
| Liquid Ether | ✅ `liquid-ether` |
| Prism | ✅ `color-bends` |
| Dark Veil | ✅ `dark-veil` |
| Silk | ✅ `silk` |
| Faulty Terminal | ✅ `faulty-terminal` |
| Light Rays | ✅ `spotlight` |
| Pixel Blast | ✅ `pixel-blast` |
| Color Bends | ✅ `color-bends` |
| Aurora | ✅ `aurora` |
| Plasma | ✅ `plasma` |
| Particles | ✅ `particle-morph`, `particles` |
| Gradient Blinds | ✅ `gradient-blinds` |
| Grid Scan | ✅ `grid-scan` |
| Beams | ✅ `light-beams` |
| Lightning | ✅ `lightning` |
| Prismatic Burst | ✅ `prismatic-burst` |
| Galaxy | ✅ `galaxy`, `galaxy-3d` |
| Dither | ✅ `pixel-blast` |
| Ripple Grid | ✅ `ripple-grid` |
| Dot Grid | ✅ `dot-grid` |
| Threads | ✅ `threads` |
| Hyperspeed | ✅ `starfield-3d`, `warp-speed` |
| Iridescence | ✅ `iridescence` |
| Waves | ✅ `waves` |
| Grid Distortion | ✅ `ripple-grid` |
| Ballpit | ✅ `ball-pit` |
| Orb | ✅ `orb` |
| Letter Glitch | ✅ `letter-glitch` |
| Grid Motion | ✅ `grid-motion` |
| Squares | ✅ `squares-grid` |
| Liquid Chrome | ✅ `liquid-metal` |
| Balatro | ✅ `swirl` |
## Magic UI

69 of 69 covered.

| Component | glowkit |
|---|---|
| Marquee | ✅ `infinite-scroll`, `marquee`, `marquee-3d`, `scroll-velocity` |
| Terminal | ✅ `terminal` |
| Hero Video Dialog | ✅ `hero-video-dialog` |
| Bento Grid | ✅ `features-bento`, `features-section` |
| Animated List | ✅ `animated-list` |
| Dock | ✅ `dock`, `floating-navbar` |
| Globe | ✅ `globe`, `globe-3d` |
| Tweet Card | ✅ `tweet-card` |
| Orbiting Circles | ✅ `integrations-section`, `orbiting-circles` |
| Avatar Circles | ✅ `animated-tooltip`, `avatar-circles` |
| Icon Cloud | ✅ `icon-cloud` |
| Animated Circular Progress Bar | ✅ `progress-ring` |
| File Tree | ✅ `tree-view` |
| Code Comparison | ✅ `code-comparison` |
| Script Copy Button | ✅ `code-block`, `script-copy` |
| Scroll Progress | ✅ `scroll-linked` |
| Lens | ✅ `lens` |
| Pointer | ✅ `smooth-cursor` |
| Smooth Cursor | ✅ `smooth-cursor` |
| Progressive Blur | ✅ `gradual-blur` |
| Dotted Map | ✅ `dotted-map` |
| Highlighter | ✅ `highlight-text` |
| Magic Card | ✅ `magic-card` |
| Neon Gradient Card | ✅ `neon-gradient-card` |
| Border Beam | ✅ `border-beam` |
| Shine Border | ✅ `shine-border` |
| Meteors | ✅ `meteors` |
| Confetti | ✅ `confetti`, `fireworks` |
| Particles | ✅ `particles` |
| Cool Mode | ✅ `particle-burst-button` |
| Scratch To Reveal | ✅ `scratch-to-reveal` |
| Blur Fade | ✅ `blur-text`, `fade-content`, `spring-presence` |
| Animated Beam | ✅ `animated-beam`, `path-drawing` |
| Number Ticker | ✅ `count-up`, `sliding-number` |
| Animated Shiny Text | ✅ `shiny-text` |
| Animated Gradient Text | ✅ `gradient-text` |
| Text Animate | ✅ `split-text`, `text-generate`, `wave-text` |
| Line Shadow Text | ✅ `line-shadow-text` |
| Aurora Text | ✅ `gradient-text` |
| Video Text | ✅ `video-text` |
| Text Reveal | ✅ `scroll-reveal-text` |
| Hyper Text | ✅ `decrypted-text`, `scramble-hover`, `shuffle-text` |
| Word Rotate | ✅ `rotating-text` |
| Typing Animation | ✅ `typewriter-text` |
| Scroll Based Velocity | ✅ `scroll-velocity` |
| Flip Text | ✅ `text-roll` |
| Box Reveal | ✅ `box-reveal` |
| Sparkles Text | ✅ `sparkles-text` |
| Morphing Text | ✅ `morphing-text` |
| Spinning Text | ✅ `circular-text` |
| Comic Text | ✅ `comic-text` |
| Animated Subscribe Button | ✅ `subscribe-button` |
| Rainbow Button | ✅ `rainbow-button` |
| Shimmer Button | ✅ `shimmer-button` |
| Ripple Button | ✅ `ripple-button` |
| Shiny Button | ✅ `shiny-button` |
| Pulsating Button | ✅ `pulsating-button` |
| Interactive Hover Button | ✅ `interactive-hover-button` |
| Warp Background | ✅ `warp-speed` |
| Flickering Grid | ✅ `flickering-grid` |
| Animated Grid Pattern | ✅ `grid-pattern` |
| Retro Grid | ✅ `faulty-terminal`, `retro-grid` |
| Ripple | ✅ `click-ripple`, `ripple` |
| Dot Pattern | ✅ `dot-grid` |
| Grid Pattern | ✅ `grid-pattern` |
| Interactive Grid Pattern | ✅ `grid-pattern`, `hex-grid` |
| Light Rays | ✅ `spotlight` |
| Animated Theme Toggler | ✅ `day-night-switch`, `theme-toggler` |
| Pixel Image | ✅ `pixel-image` |
## Aceternity UI

79 of 79 covered.

| Component | glowkit |
|---|---|
| 3D Card | ✅ `tilt-card` |
| 3D Marquee | ✅ `marquee-3d` |
| 3D Pin | ✅ `pin-3d` |
| Animated Modal | ✅ `dialog` |
| Animated Testimonials | ✅ `animated-testimonials`, `testimonials-section` |
| Animated Tooltip | ✅ `animated-tooltip` |
| Apple Cards Carousel | ✅ `apple-cards-carousel` |
| Aurora Background | ✅ `aurora` |
| Background Beams | ✅ `background-beams` |
| Background Beams With Collision | ✅ `background-beams` |
| Background Boxes | ✅ `background-boxes` |
| Background Gradient | ✅ `animated-gradient-border` |
| Background Gradient Animation | ✅ `gradient-blobs` |
| Background Lines | ✅ `background-lines` |
| Bento Grid | ✅ `features-bento` |
| Canvas Reveal Effect | ✅ `canvas-reveal` |
| Card Hover Effect | ✅ `card-hover-effect` |
| Card Spotlight | ✅ `spotlight-card` |
| Card Stack | ✅ `stack-cards` |
| Carousel | ✅ `carousel` |
| Colourful Text | ✅ `colorful-text` |
| Compare | ✅ `compare-slider` |
| Container Scroll Animation | ✅ `container-scroll` |
| Cover | ✅ `cover` |
| Codeblock | ✅ `code-block` |
| Direction Aware Hover | ✅ `direction-aware-hover` |
| Draggable Card | ✅ `drag` |
| Evervault Card | ✅ `encrypted-card` |
| Expandable Card | ✅ `expandable-card`, `shared-element` |
| File Upload | ✅ `file-dropzone` |
| Flip Words | ✅ `rotating-text` |
| Floating Dock | ✅ `dock` |
| Floating Navbar | ✅ `floating-navbar`, `header-section` |
| Focus Cards | ✅ `focus-cards` |
| Following Pointer | ✅ `following-pointer` |
| Glare Card | ✅ `glare-card` |
| Glowing Effect | ✅ `glowing-border-card` |
| Glowing Stars | ✅ `glowing-stars` |
| Google Gemini Effect | ✅ `gemini-effect` |
| Grid and Dot Backgrounds | ✅ `dot-grid`, `grid-pattern` |
| Hero Highlight | ✅ `highlight-text` |
| Hero Parallax | ✅ `hero-parallax` |
| Hover Border Gradient | ✅ `hover-border-gradient` |
| Images Slider | ✅ `images-slider` |
| Infinite Moving Cards | ✅ `marquee` |
| Lamp Effect | ✅ `lamp` |
| Layout Grid | ✅ `gallery-section` |
| Lens | ✅ `lens` |
| Link Preview | ✅ `link-preview` |
| Macbook Scroll | ✅ `laptop-mockup` |
| Meteors | ✅ `meteors` |
| Moving Border | ✅ `moving-border-button` |
| Multi Step Loader | ✅ `multi-step-loader` |
| Navbar Menu | ✅ `navbar`, `navigation-menu` |
| Parallax Scroll | ✅ `scroll-linked` |
| Placeholders And Vanish Input | ✅ `vanish-input` |
| Pointer Highlight | ✅ `pointer-highlight` |
| Resizable Navbar | ✅ `resizable-navbar` |
| Shooting Stars | ✅ `shooting-stars`, `starfield-3d` |
| Sidebar | ✅ `shell-sidebar`, `sidebar` |
| Signup Form | ✅ `signup-page` |
| Sparkles | ✅ `particle-morph`, `particles`, `sparkles-text` |
| Spotlight | ✅ `spotlight` |
| Sticky Banner | ✅ `sticky-banner` |
| Sticky Scroll Reveal | ✅ `sticky-scroll-reveal` |
| SVG Mask Effect | ✅ `svg-mask-effect` |
| Tabs | ✅ `animated-tabs` |
| Text Generate Effect | ✅ `text-generate` |
| Text Hover Effect | ✅ `text-hover-effect` |
| Text Reveal Card | ✅ `text-reveal-card` |
| Timeline | ✅ `timeline` |
| Tracing Beam | ✅ `tracing-beam` |
| Typewriter Effect | ✅ `typewriter-text` |
| Vortex | ✅ `vortex` |
| Wavy Background | ✅ `wavy-background` |
| Wobble Card | ✅ `wobble-card` |
| World Map | ✅ `dotted-map` |
| Stateful Button | ✅ `stateful-button` |
| Gradient text | ✅ `gradient-text` |
## Motion Primitives

33 of 33 covered.

| Component | glowkit |
|---|---|
| Accordion | ✅ `accordion` |
| Animated Background | ✅ `animated-tabs`, `navbar`, `tabs` |
| Animated Group | ✅ `stagger-variants` |
| Animated Number | ✅ `count-up` |
| Border Trail | ✅ `border-beam` |
| Carousel | ✅ `carousel` |
| Cursor | ✅ `smooth-cursor` |
| Dialog | ✅ `dialog` |
| Disclosure | ✅ `collapsible` |
| Dock | ✅ `dock` |
| Glow Effect | ✅ `shiny-button` |
| Image Comparison | ✅ `compare-slider` |
| In View | ✅ `spring-presence` |
| Infinite Slider | ✅ `marquee` |
| Magnetic | ✅ `magnet`, `magnetic-button` |
| Morphing Dialog | ✅ `expandable-card`, `shared-element` |
| Morphing Popover | ✅ `popover` |
| Progressive Blur | ✅ `gradual-blur` |
| Scroll Progress | ✅ `scroll-linked` |
| Sliding Number | ✅ `sliding-number` |
| Spinning Text | ✅ `circular-text` |
| Spotlight | ✅ `spotlight-card` |
| Text Effect | ✅ `blur-text`, `split-text`, `typewriter-text` |
| Text Loop | ✅ `rotating-text` |
| Text Morph | ✅ `morphing-text` |
| Text Roll | ✅ `text-roll` |
| Text Scramble | ✅ `decrypted-text` |
| Text Shimmer | ✅ `shiny-text` |
| Text Shimmer Wave | ✅ `wave-text` |
| Tilt | ✅ `tilt-card` |
| Toolbar Dynamic | ✅ `morphing-island` |
| Toolbar Expandable | ✅ `morphing-island` |
| Transition Panel | ✅ `spring-presence` |
## shadcn/ui

58 of 58 covered.

| Component | glowkit |
|---|---|
| Accordion | ✅ `accordion` |
| Alert | ✅ `alert` |
| Alert Dialog | ✅ `alert-dialog` |
| Aspect Ratio | ✅ `aspect-ratio` |
| Avatar | ✅ `avatar` |
| Badge | ✅ `badge` |
| Breadcrumb | ✅ `breadcrumb` |
| Button | ✅ `button` |
| Button Group | ✅ `button-group` |
| Calendar | ✅ `calendar` |
| Card | ✅ `card`, `hover-lift-card` |
| Carousel | ✅ `carousel` |
| Chart | ✅ `line-chart` |
| Checkbox | ✅ `checkbox` |
| Collapsible | ✅ `collapsible` |
| Combobox | ✅ `select` |
| Command | ✅ `command-palette`, `search-input` |
| Context Menu | ✅ `context-menu` |
| Data Table | ✅ `data-table` |
| Date Picker | ✅ `calendar` |
| Dialog | ✅ `dialog` |
| Drawer | ✅ `sheet` |
| Dropdown Menu | ✅ `dropdown-menu` |
| Empty | ✅ `empty-state` |
| Field | ✅ `form` |
| Form | ✅ `form` |
| Hover Card | ✅ `hover-card` |
| Input | ✅ `input` |
| Input Group | ✅ `input` |
| Input OTP | ✅ `otp-input` |
| Item | ✅ `item` |
| Kbd | ✅ `kbd` |
| Label | ✅ `form` |
| Menubar | ✅ `menubar` |
| Navigation Menu | ✅ `navigation-menu` |
| Pagination | ✅ `pagination` |
| Popover | ✅ `popover` |
| Progress | ✅ `bar-list`, `progress` |
| Radio Group | ✅ `radio-group` |
| Resizable | ✅ `resizable` |
| Scroll Area | ✅ `scroll-area` |
| Select | ✅ `select` |
| Separator | ✅ `kbd` |
| Sheet | ✅ `sheet` |
| Sidebar | ✅ `shell-sidebar`, `sidebar` |
| Skeleton | ✅ `skeleton` |
| Slider | ✅ `slider` |
| Sonner | ✅ `toast` |
| Spinner | ✅ `spinner` |
| Switch | ✅ `switch` |
| Table | ✅ `data-table`, `table` |
| Tabs | ✅ `animated-tabs`, `tabs` |
| Textarea | ✅ `textarea` |
| Toast | ✅ `toast` |
| Toggle | ✅ `toggle-group` |
| Toggle Group | ✅ `toggle-group` |
| Tooltip | ✅ `tooltip` |
| Typography | ✅ `blog-post-page` |
## Vercel AI Elements

33 of 33 covered.

| Component | glowkit |
|---|---|
| Actions | ✅ `ai-chat-message` |
| Artifact | ✅ `ai-artifact` |
| Branch | ✅ `ai-chat-message` |
| Canvas | ✅ `ai-workflow` |
| Chain of Thought | ✅ `ai-reasoning` |
| Checkpoint | ✅ `ai-model-selector` |
| Code Block | ✅ `ai-markdown` |
| Confirmation | ✅ `ai-confirmation` |
| Connection | ✅ `node-editor` |
| Context | ✅ `ai-context-meter` |
| Controls | ✅ `node-editor` |
| Conversation | ✅ `ai-conversation` |
| Edge | ✅ `ai-workflow` |
| Image | ✅ `ai-image-gen` |
| Inline Citation | ✅ `ai-sources` |
| Loader | ✅ `ai-loaders` |
| Message | ✅ `ai-chat-message` |
| Model Selector | ✅ `ai-model-selector` |
| Node | ✅ `ai-workflow` |
| Open In Chat | ✅ `ai-open-in-chat` |
| Panel | ✅ `node-editor` |
| Plan | ✅ `ai-task-list` |
| Prompt Input | ✅ `ai-prompt-input` |
| Queue | ✅ `ai-task-list` |
| Reasoning | ✅ `ai-reasoning` |
| Response | ✅ `ai-markdown`, `ai-streaming-text` |
| Shimmer | ✅ `ai-loaders` |
| Sources | ✅ `ai-sources` |
| Suggestion | ✅ `ai-suggestions` |
| Task | ✅ `ai-task-list` |
| Tool | ✅ `ai-tool-call` |
| Toolbar | ✅ `ai-toolbar` |
| Web Preview | ✅ `ai-web-preview` |
## prompt-kit

19 of 19 covered.

| Component | glowkit |
|---|---|
| Prompt Input | ✅ `ai-prompt-input` |
| Message | ✅ `ai-chat-message` |
| Markdown | ✅ `ai-markdown` |
| Code Block | ✅ `ai-markdown` |
| Chat Container | ✅ `ai-conversation` |
| Scroll Button | ✅ `ai-conversation` |
| Loader | ✅ `ai-loaders` |
| Prompt Suggestion | ✅ `ai-suggestions` |
| Response Stream | ✅ `ai-streaming-text` |
| Reasoning | ✅ `ai-reasoning` |
| File Upload | ✅ `ai-prompt-input` |
| JSX Preview | ✅ `ai-jsx-preview` |
| Tool | ✅ `ai-tool-call` |
| Source | ✅ `ai-sources` |
| Steps | ✅ `ai-reasoning` |
| Text Shimmer | ✅ `ai-loaders` |
| Thinking Bar | ✅ `ai-reasoning` |
| Feedback Bar | ✅ `ai-chat-message` |
| Image | ✅ `ai-image-gen` |
## ElevenLabs UI

16 of 16 covered.

| Component | glowkit |
|---|---|
| Orb | ✅ `ai-voice-orb` |
| Waveform | ✅ `ai-waveform` |
| Live Waveform | ✅ `ai-waveform` |
| Bar Visualizer | ✅ `ai-waveform` |
| Voice Button | ✅ `ai-waveform` |
| Voice Picker | ✅ `ai-model-selector` |
| Conversation | ✅ `ai-conversation` |
| Message | ✅ `ai-chat-message` |
| Response | ✅ `ai-streaming-text` |
| Shimmering Text | ✅ `ai-loaders` |
| Audio Player | ✅ `ai-audio-player` |
| Matrix | ✅ `ai-dot-matrix` |
| Mic Selector | ✅ `ai-mic-selector` |
| Speech Input | ✅ `ai-speech-input` |
| Scrub Bar | ✅ `ai-scrub-bar` |
| Transcript Viewer | ✅ `ai-transcript-viewer` |
## Evil Charts

6 of 6 covered (chart families; each is covered by any glowkit style of that family).

| Component | glowkit |
|---|---|
| Area Chart | ✅ `area-chart` |
| Bar Chart | ✅ `bar-chart` |
| Line Chart | ✅ `line-chart` |
| Pie Chart | ✅ `pie-chart` |
| Radar Chart | ✅ `radar-chart` |
| Radial Chart | ✅ `radial-bar-chart` |
## MVP Blocks

25 of 25 covered (section types (their catalog is organized by section)).

| Component | glowkit |
|---|---|
| Hero | ✅ `hero-section` |
| Features | ✅ `features-section` |
| Bento Grid | ✅ `features-bento` |
| Pricing | ✅ `pricing-section` |
| Testimonials | ✅ `testimonials-section` |
| CTA | ✅ `cta-section` |
| Footer | ✅ `footer-section` |
| Header | ✅ `header-section` |
| About | ✅ `about-section` |
| Contact | ✅ `contact-section` |
| FAQ | ✅ `faq-section` |
| Team | ✅ `team-section` |
| Waitlist | ✅ `newsletter-section`, `waitlist-page` |
| Login | ✅ `login-page` |
| Signup | ✅ `signup-page` |
| Not Found | ✅ `not-found-page` |
| Dashboard | ✅ `app-analytics` |
| Stats | ✅ `stats-section` |
| Logo Cloud | ✅ `logo-cloud` |
| Newsletter | ✅ `newsletter-section` |
| Blog | ✅ `blog-section` |
| Changelog | ✅ `changelog-section` |
| Integrations | ✅ `integrations-section` |
| Gallery | ✅ `gallery-section` |
| Timeline | ✅ `about-section` |
## React Flow

12 of 12 covered.

| Component | glowkit |
|---|---|
| Background | ✅ `infinite-canvas` |
| Controls | ✅ `node-editor` |
| MiniMap | ✅ `node-editor` |
| Node Toolbar | ✅ `ai-toolbar`, `node-editor` |
| Edge Label | ✅ `node-editor` |
| Custom Nodes | ✅ `shader-graph` |
| Custom Edges | ✅ `node-editor` |
| Animated Edges | ✅ `agent-pipeline` |
| Selection | ✅ `node-editor` |
| Snap to Grid | ✅ `node-editor` |
| Auto Layout | ✅ `node-editor` |
| Undo Redo | ✅ `node-editor` |
## Tremor

12 of 12 covered.

| Component | glowkit |
|---|---|
| Area Chart | ✅ `area-chart` |
| Bar Chart | ✅ `bar-chart` |
| Line Chart | ✅ `line-chart` |
| Donut Chart | ✅ `pie-chart` |
| Bar List | ✅ `bar-list` |
| Spark Chart | ✅ `sparkline` |
| Tracker | ✅ `tracker` |
| Category Bar | ✅ `bar-list` |
| Progress Bar | ✅ `bar-list` |
| Progress Circle | ✅ `gauge`, `tracker` |
| Combo Chart | ✅ `combo-chart` |
| Scatter Chart | ✅ `scatter-chart` |

## Also covers

* **21st.dev**: Pointer trail
* **8bitcn**: Button
* **Animata**: Wave Text
* **Animate UI**: Accordion, Alert Dialog, Bottom Navigation, Button, Checkbox, Collapsible, Context Menu, Counter, Counting Number, Dialog, Dropdown Menu, Hover Card, Input, Input OTP, Loading, Menubar, Popover, Radio Group, Sheet, Sidebar, Slider, Switch, Tabs, Toast, Toggle Group, Tooltip
* **Anime.js**: timeline
* **Apple**: Dynamic Island
* **Apple Mail**: Three-pane layout
* **Apple Music**: Playlist view
* **Asana**: List + Board
* **Attio**: Board view, Pipeline board
* **Blender**: Shader editor
* **CSS**: @keyframes, Marching ants, conic border, flip card, offset-path
* **CapCut**: Timeline
* **ChatGPT**: App layout
* **Claude**: Artifacts panel
* **Clerk**: Sign-in
* **CodePen**: Liquid button, Matrix Digital Rain, Neon button
* **Codrops**: Image Trail, elastic trails
* **Cult UI**: Animated Link, Animated link, Dynamic Island, Expandable Tabs, Floating Nav, Glitch Text, Text Roll, Text Scramble, Text Wave
* **DaVinci Resolve**: Edit page
* **Datadog**: Live tail
* **Dice UI**: Data Table, File Upload, Stepper, Tags Input
* **Discord**: Server rail, Text channel
* **Docusaurus**: TOC
* **Dropbox**: Files
* **ECharts**: Bar3D
* **Excalidraw**: Canvas, Library
* **Fancy Components**: Scramble Hover
* **FigJam**: Whiteboard
* **Figma**: Canvas navigation, Properties panel
* **Flowise**: Chatflow canvas
* **Framer Motion**: AnimatePresence, Reorder.Group, animate sequences, drag, dragConstraints / dragElastic / dragMomentum, easing functions, exit animations, keyframes, layout prop, layoutId, motion.div, path animations, path morphing, pathLength, shared layout, shared layout animations, staggerChildren / delayChildren, useScroll / useTransform, useSpring, useSpring chains, variants, whileHover / whileTap, whileInView
* **Fumadocs**: TOC
* **GSAP**: Draggable + Inertia, DrawSVG, Ease Visualizer, Flip plugin, MorphSVG, MotionPathPlugin, ScrollTrigger, Timeline, elastic ease, keyframes, stagger
* **GitHub**: contribution graph
* **Glassmorphism**: Card
* **Google Calendar**: Week view
* **Google Drive**: My Drive
* **Hover.dev**: Arrow button, Fill button, Glass card, Glow button, Gradient button, Lift card, Magnetic button, Underline links
* **Jira**: Board, Kanban
* **Josh Comeau**: 3D button
* **Kendo**: OrgChart
* **Kibo UI**: AI Image, AI Loader, AI Response, AI Sources, AI Tool, Announcement, Avatar Stack, Banner, Breadcrumb, Card, Code Block, Color Picker, Combobox, Command, Description List, Dropzone, Empty, KPI, Kanban, Link, Mini Calendar, Number field, Pagination, Pill, Progress, Rating, Search, Status, Stepper, Table, Table of Contents, Tags, Timeline, Tour / Coachmark, Tree
* **Kokonut UI**: AI Input
* **LangGraph Studio**: Graph view
* **Linear**: Board, Board view, Dual navigation, Project issues, Timeline, Upgrade prompt
* **Lucidchart**: Org chart
* **MUI**: Stepper
* **Mantine**: Stepper
* **Material**: Bottom navigation, Chips, Navigation rail, Ripple
* **Material 3**: Navigation Bar
* **Media Chrome**: Time Range
* **Medusa**: Admin dashboard
* **Mercury**: Accounts home
* **Mintlify**: Docs layout
* **Miro**: Board, Mind map
* **Motion One**: hover / press, keyframes, spring, stagger, timeline
* **NES.css**: Button
* **Neobrutalism**: Button
* **Nextra**: Docs theme, TOC
* **Nivo**: Bar, Bar + Line, Calendar, Candlestick, Funnel, HeatMap, Line, Pie, Radar, Radial Bar, ScatterPlot, Stream, Swarm/Bubble, TimeRange, TreeMap
* **Notion**: Timeline view
* **Origin UI**: Accordion, Alert Dialog, Alerts, Avatar, Avatar group, Badges, Banner, Breadcrumb, Button groups, Buttons, Cards, Checkbox, Circular progress, Color picker, Command, Date range picker, Dialog, Drawer, Dropdown, Empty states, File upload, Form fields, Hover Card, Input with tags, Inputs, Kbd, Key-value, Link, List items, Navbar, Navbar with dropdown, Number input, OTP input, Pagination, Password input, Password strength, Popover, Progress, Radio cards, Range slider, Rating, Search input, Segmented control, Select, Sidebar, Skeleton, Slider, Spinner, Stats, Stepper, Switch, Tab bar, Table, Tables, Tabs, Tags / Chips, Textarea, Textarea with character limit, Timeline, Toast, Tooltip, Tour, Tree
* **Paper Shaders**: Grain Gradient, Mesh Gradient
* **Pipedrive**: Pipeline
* **Plotly**: 3D bars
* **Premiere Pro**: Timeline
* **Radix**: Context Menu, Dialog, Dropdown Menu, Hover Card, Menubar, Navigation Menu, Popover, Tooltip
* **Radix UI**: Aspect Ratio, Scroll Area
* **React Spring**: useSpring
* **Recharts**: AreaChart, BarChart, ComposedChart, LineChart, PieChart, RadarChart, RadialBarChart, ScatterChart, Tiny Line/Area/Bar, Treemap
* **Remotion**: Timeline
* **RetroUI**: Button, Pixel button
* **SVAR**: React Gantt
* **Shepherd.js**: Tour
* **Shopify Polaris**: Orders index, Product index
* **Slack**: Channel view
* **Sonner**: Toaster, toast stack
* **SpinKit**: Spinners
* **Spline**: 3D hero, 3D text, Terrain hero, cube hero, planets hero, science hero, space hero
* **Spotify**: Desktop player
* **Streamdown**: Markdown
* **Stripe**: Customer portal, Developers dashboard, globe hero
* **Supabase**: API settings, Logs explorer
* **Superhuman**: Inbox
* **Swiper**: Coverflow
* **Tailark**: About, Announcement, Blog, Call to Action, Contact, FAQs, Features, Footer, Header, Hero Section, Hero with Image, Integrations, Landing, Login, Logo Cloud, Newsletter, Not Found, Pricing, Sign Up, Stats, Steps, Team, Testimonials
* **Tailwind UI**: Billing settings, CRM, Calendars, Chat, Description lists, Ecommerce admin, Empty states, File grid, Finance dashboard, Kanban, Mobile app shell, Product lists, Project list, Settings screens, Sidebar layout, Sign-in forms, Stacked layout, Stats + charts dashboard, Sticky header
* **TanStack Table**: Virtualized
* **TradingView Lightweight Charts**: Candlestick
* **Trello**: Board
* **Twenty**: People table
* **Uiverse**: 3D button, Animated border button, Animated border card, Arrow button, Confetti button, Dashed border animation, Day night toggle, File input, Glass card, Glow button, Gradient border button, Hover card, Liquid fill button, Loaders, Loading button, Loading text, Magnetic button, Neon button, Progress bars, Pulse button, Ripple button, Shimmer button, Skeleton loaders, Star rating
* **Unovis**: Area, Candlestick, Donut, Funnel, Grouped Bar / Stacked Bar, Heatmap, Line, Scatter, Treemap
* **Unreal**: Material editor
* **Vaul**: Drawer
* **Vercel**: Empty states, Runtime logs, Team settings, Usage
* **Vercel AI SDK**: Generative UI, Tool approval
* **Vidstack**: Time Slider
* **Whimsical**: Mind maps
* **XMind**: Mind map
* **assistant-ui**: Artifacts, Branch Picker, Composer, Context usage, Human-in-the-loop approval, Markdown Text, Message, Model Picker, Reasoning, Thread, Thread Suggestions, Thread Viewport, Tool UI, Tool progress
* **cmdk**: Command Menu
* **dnd kit**: Sortable
* **dnd-kit**: Sortable multiple containers
* **draw.io**: Flowchart shapes
* **drei**: ContactShadows, OrbitControls, Stage, Text
* **iOS**: Segmented Control, Tab bar
* **llm-ui**: Streaming
* **n8n**: Workflow canvas
* **react-resizable-panels**: PanelGroup
* **react-three-fiber**: Canvas, data viz
* **rough.js**: Sketchy shapes
* **shadcn blocks**: Announcement Bar, Navbar
* **shadcn-chatbot-kit**: Chat, Chat Message, Markdown Renderer, Message Input, Prompt Suggestions, Typing Indicator
* **shadcnblocks**: 404, Banner, Blog, Blog Post, CTA, Changelog, Coming Soon, Compare, Contact, Error page, FAQ, Feature, Footer, Forgot Password, Gallery, Hero, Hero with Screenshot, Landing Page, Multi-step form, Order confirmation, Pricing, Pricing Page, Process, Profile, Stats, Team, Timeline
* **simple-ai**: Chat, Chat Input, Workflow
* **three.js**: Bloom torus knot, DNA helix, Galaxy generator, Points morph hero, Rubik's cube, Scene + MeshStandardMaterial, Solar system, Starfield warp, Synthwave terrain, TextGeometry
* **tldraw**: Editor, Geo shapes, Infinite canvas
* **tsParticles**: Fireworks, Links
* **v0**: Preview panel
