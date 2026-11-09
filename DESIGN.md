# Desktop design contract

Mode: Operate. Preserve the dashboard's Survey Sheet design.

The supplied dashboard design system and its Button, Input, Table, StatusBadge, InfoField and PageHeader components are the visual authority. The desktop translates their semantic tokens into Rune, rather than treating the native app as a website.

- Light: paper background, deeper paper sidebar, ink foreground. Dark: night chart background with warm light foreground.
- One-pixel rules separate surfaces. No shadows. Corners use 2, 3 and 4 point radii.
- Figtree for controls and prose, Mona Sans for page titles, JetBrains Mono for machine values. Embedded font assets retain their licenses.
- Spacing: 4, 8, 12, 16, 24, 32 points. Body 14 points, labels 12, section headings 18, page titles 28.
- Status: square success for Active, triangular warning, circular information, diamond failure, filled neutral dot for Ready, and an outlined neutral ring for inactive states. Ready means listening without an open session. Every mark has a text label.
- Primary buttons use foreground/background contrast. Orange belongs to selection and focus. Destructive actions require explicit confirmation.
- Theme follows the system unless the user chooses light or dark. Motion uses Rune's virtual clock and reduced-motion support.

Layout: Main window uses a split view with sidebar navigation, a toolbar, and one scrolling content region. The tunnels table owns its row scrolling; the selected tunnel's details follow below the table. No horizontal overflow at the minimum supported desktop size.

Layout: Add tunnel is a modal form with a single scrolling body and visible Cancel and Save actions. Organization selects the server list; server selects the scanned ports. The manual target remains available.

The HTTPS hostname preview stays outside the scrolling form body. A fading edge and native scrollbar reveal overflow. Kind labels are uppercase; the wire values remain lowercase. Text action labels align to their surrounding content edge.

Layout: Menu-bar panel is 340 points wide with one scrolling list and a fixed footer. Approvals precede server groups. Each tunnel has state, toggle and endpoint actions.

Priorities: tunnel state and endpoint first, server and transport second, traffic and diagnostics third. No decorative metrics or promotional copy. Required data remains present through empty, loading, disconnected, signed-out and error states.

The sidebar is the main window's only product wordmark. The toolbar reserves its trailing slot for real status, such as a disconnected daemon. The selected table row and sidebar use the same selection token. Switches describe a state, "Tunnel up", with a light thumb in both appearances.

Native desktop acceptance: window at 1100 x 760 and minimum 760 x 620; panel at 340 x 620. A 390 point compact capture checks narrow content behavior, but this is not an iPhone app and mobile safe areas do not apply. Light and dark are captured for every required view and state. Rune PNGs and element trees are the visual evidence; browser-only detectors do not apply to the native renderer.
