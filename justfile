name := "keyboard-music"

setup:
  if   command -v pacman       >/dev/null 2>&1 /dev/null; then sudo pacman -S --needed alure openal gcc; fi

make:
  gcc -O2 -Wall main.c `pkg-config --libs openal alure xtst x11` -lm -o {{name}}

run: make
  ./{{name}}

i3:
