# dmgbuild settings for the Reoclo install window.
# Run from the repository root:
#   dmgbuild -s assets/dmg/settings.py -D app=/path/to/Reoclo.app Reoclo out.dmg

app = defines["app"]

format = "UDZO"
files = [app]
symlinks = {"Applications": "/Applications"}
icon = "assets/Reoclo.icns"
background = "assets/dmg/background.png"

show_status_bar = False
show_tab_view = False
show_toolbar = False
show_pathbar = False
show_sidebar = False
window_rect = ((200, 140), (660, 420))

default_view = "icon-view"
show_icon_preview = False
arrange_by = None
label_pos = "bottom"
text_size = 13
icon_size = 128
icon_locations = {
    "Reoclo.app": (175, 178),
    "Applications": (485, 178),
}
