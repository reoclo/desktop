# Desktop design contract

Mode: Operate. Preserve the dashboard's Survey Sheet design.

The supplied dashboard design system and its Button, Input, Table, StatusBadge, InfoField and PageHeader components are the visual authority. The desktop translates their semantic tokens into Rune, rather than treating the native app as a website.

- Light: paper background, deeper paper sidebar, ink foreground. Dark: night chart background with warm light foreground.
- One-pixel rules separate surfaces. No shadows. Corners use 2, 3 and 4 point radii.
- Figtree for controls and prose, Mona Sans for page titles, JetBrains Mono for machine values. Embedded font assets retain their licenses.
- Spacing: 4, 8, 12, 16, 24, 32 points. Body 14 points, labels 12, section headings 18, page titles 28.
- Status: square success for Active, triangular warning, circular information, diamond failure, filled neutral dot for Ready, and an outlined neutral ring for inactive states. Ready means listening without an open session. Every mark has a text label.
- Four button ranks: primary (flat foreground/background contrast), secondary (a one-point rule), danger (a one-point danger rule, for an action that opens a confirmation), and destructive (filled danger, only for the button that confirms). Offer the action the current state allows rather than disabling the others. In a dialog the primary action is rightmost, with Cancel to its left. Orange belongs to selection and focus. Destructive actions require explicit confirmation.
- Theme follows the system unless the user chooses light or dark. Motion uses Rune's virtual clock and reduced-motion support.

Layout: Main window uses a split view with sidebar navigation, a toolbar, and one scrolling content region. The tunnels table owns its row scrolling and sizes to its rows up to eight; each row has its own toggle, because bringing a tunnel up is the primary action. The selected tunnel's details follow below the table. Pending approvals sit above the table, because an agent is blocked on them. Tunnels running in a terminal sit between them, with their real ports and a confirmed Stop, and they show even with no saved tunnels. A TCP forward or saved TCP tunnel offers Add HTTPS, which opens the Add tunnel form prefilled for the same target. No horizontal overflow at the minimum supported desktop size.

Layout: Add tunnel is a modal form with a single scrolling body and visible Cancel and Save actions. Organization selects the server list; server selects the scanned ports. The manual target remains available.

The form is two columns and fits without scrolling in its usual state, with Kind first because it decides the rest. The HTTPS hostname preview stays outside the scrolling form body. A fading edge and native scrollbar reveal overflow when a warning or error adds height. Kind labels are uppercase; the wire values remain lowercase. Selects and inputs share one height and rule. An HTTPS hostname is an optional subdomain on a trusted domain chosen from a list; without a trusted CA the form offers to trust it and stays open so it can be saved next. Certificates trusts another domain, and Hosts registers and removes hostnames of your own. The port 443 relay has a switch that reinstalls the helper after a confirmation, and says when another program holds port 443. Every helper reinstall keeps the relay and the trusted domains outside .localhost. A trusted domain you added has Remove, which reissues the CA without it after a confirmation; a stale CA offers Trust new CA. Form dropdowns can be searched, and every floating menu has a one-point rule so it reads against the surface it opens over.

Layout: Menu-bar panel is 340 points wide with one scrolling list and a fixed footer. Approvals precede server groups. Each tunnel takes two lines: name, state, icon actions and toggle, then the endpoint across the full width. The panel header carries the mark and the wordmark. The menu bar item follows a setting: icon, icon and name, icon and active count, or icon and a meter of one bar per tunnel. Pending approvals show in every style.

Priorities: tunnel state and endpoint first, server and transport second, traffic and diagnostics third. No decorative metrics or promotional copy. Required data remains present through empty, loading, disconnected, signed-out and error states.

The sidebar is the main window's only product wordmark, and it carries the hexagon mark. The toolbar reserves its trailing slot for real status: a disconnected daemon, work in progress, or the last confirmation. Failures appear under the page title, where they cannot be scrolled past. Mono is for machine values, never prose. Status words always carry their mark. The selected table row and sidebar use the same selection token. Switches describe a state, "Tunnel up", with a light thumb in both appearances, on a track at least 3:1 against it.

Native desktop acceptance: window at 1100 x 760 and minimum 760 x 620; panel at 340 x 620. A 390 point compact capture checks narrow content behavior, but this is not an iPhone app and mobile safe areas do not apply. Light and dark are captured for every required view and state. Rune PNGs and element trees are the visual evidence; browser-only detectors do not apply to the native renderer.
