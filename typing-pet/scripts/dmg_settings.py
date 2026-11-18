# dmgbuild 설정 (scripts/installer.sh 가 -D 로 경로를 넘긴다).
# 창 내용 768x512, 배경 art/_app/final/dmg_bg.png 의 빈 동그라미 두 개에 앱과 응용 프로그램 바로가기를 놓는다.
app = defines["app"]  # noqa: F821 (dmgbuild 가 넣어 준다)
background = defines["bg"]  # noqa: F821
icon = defines["icon"]  # noqa: F821

format = "UDZO"
files = [app]
symlinks = {"Applications": "/Applications"}
icon_locations = {
    "TypingPet.app": (196, 176),
    "Applications": (564, 176),
}
# 높이 512 + 제목 막대 32 (창 크기는 제목 막대를 포함한다)
window_rect = ((200, 140), (768, 544))
default_view = "icon-view"
icon_size = 112
text_size = 13
show_status_bar = False
show_tab_view = False
show_toolbar = False
show_pathbar = False
show_sidebar = False
hide_extensions = ["TypingPet.app"]
