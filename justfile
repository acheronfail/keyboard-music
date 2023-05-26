make:
  gcc -O2 -Wall main.c `pkg-config --libs openal alure xtst x11` -lm -o main

run: make
  ./main
