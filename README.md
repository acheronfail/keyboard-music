# Keyboard Music

A dumb little thing I made for when my kids interrupt me at work!

## Usage

Run it, then use your keyboard.

Goes nicely with an i3 config that's something like this:

```conf
bindsym --release Mod4+shift+m mode "music", exec ~/src/keyboard-music/keyboard-music
mode "music" {
  bindsym a            nop
  bindsym b            nop
  bindsym c            nop
  bindsym d            nop
  bindsym e            nop
  bindsym f            nop
  bindsym g            nop
  bindsym h            nop
  bindsym i            nop
  bindsym j            nop
  bindsym k            nop
  bindsym l            nop
  bindsym m            nop
  bindsym n            nop
  bindsym o            nop
  bindsym p            nop
  bindsym q            nop
  bindsym r            nop
  bindsym s            nop
  bindsym t            nop
  bindsym u            nop
  bindsym v            nop
  bindsym w            nop
  bindsym x            nop
  bindsym y            nop
  bindsym z            nop
  bindsym 1            nop
  bindsym 2            nop
  bindsym 3            nop
  bindsym 4            nop
  bindsym 5            nop
  bindsym 6            nop
  bindsym 7            nop
  bindsym 8            nop
  bindsym 9            nop
  bindsym 0            nop
  bindsym F1           nop
  bindsym F2           nop
  bindsym F3           nop
  bindsym F4           nop
  bindsym F5           nop
  bindsym F6           nop
  bindsym F7           nop
  bindsym F8           nop
  bindsym F9           nop
  bindsym F10          nop
  bindsym F11          nop
  bindsym F12          nop
  bindsym grave        nop
  bindsym minus        nop
  bindsym equal        nop
  bindsym BackSpace    nop
  bindsym insert       nop
  bindsym bracketleft  nop
  bindsym bracketright nop
  bindsym backslash    nop
  bindsym semicolon    nop
  bindsym apostrophe   nop
  bindsym comma        nop
  bindsym period       nop
  bindsym Return       nop
  bindsym Delete       nop
  bindsym Caps_Lock    nop
  bindsym Tab          nop
  bindsym Escape       nop
  bindsym Up           nop
  bindsym Left         nop
  bindsym Down         nop
  bindsym Right        nop

  bindsym --release Mod4+shift+m mode "default", exec killall keyboard-music
}
```