#include <AL/al.h>
#include <AL/alc.h>
#include <X11/XKBlib.h>
#include <X11/extensions/record.h>
#include <limits.h>
#include <math.h>
#include <stdio.h>
#include <stdbool.h>

#define SECOND 1
#define SAMPLING_HZ 44100
#define BUFFER_LENGTH (SECOND * SAMPLING_HZ)
#define SOUND_HZ 440

#define NOTES 128

static ALuint buf[512] = {0};
static ALuint src[512] = {0};

static int stack_pointer = -1;
static int stack[512];

static Display *dpy = NULL;
static XRecordContext rc;

int handle_input(int code, int press) {
  if (press) {
    if (stack_pointer >= 0) {
      alSourceStop(src[stack[stack_pointer]]);
    }

    stack_pointer++;
    stack[stack_pointer] = code;
    alSourcePlay(src[stack[stack_pointer]]);
  } else {
    alSourceStop(src[stack[stack_pointer]]);
    stack_pointer--;
    if (stack_pointer >= 0) {
      alSourcePlay(src[stack[stack_pointer]]);
    }
  }

  return 0;
}

// TODO: terminate loop if escape pressed: XRecordDisableContext
void key_pressed_cb(XPointer arg, XRecordInterceptData *d) {
  if (d->category != XRecordFromServer)
    return;

  int key = ((unsigned char *)d->data)[1];
  int type = ((unsigned char *)d->data)[0] & 0x7F;
  int repeat = d->data[2] & 1;

  key -= 8; /* X code to scan code? */

  if (!repeat) {

    switch (type) {
    case KeyPress:
      if (key == 1) {
        // TODO: don't exit here, but clean up properly
        exit(0);
        // FIXME: why does XRecordDisableContext never return
        // https://www.x.org/releases/X11R7.7/doc/libXtst/recordlib.html#XRecordDisableContext
        // https://gitlab.freedesktop.org/xorg/lib/libxtst/-/issues/1
        // https://stackoverflow.com/a/69717395/5552584
        XRecordDisableContext(dpy, rc);
        XSync(dpy, false);
        XFlush(dpy);
        return;
      }
      handle_input(key, 1);
      break;
    case KeyRelease:
      handle_input(key, 0);
      break;
    case ButtonPress:
      if (key == -5 || key == -7)
        handle_input(0xff, 1);
      break;
    case ButtonRelease:
      if (key == -5 || key == -7)
        handle_input(0xff, 0);
      break;
    default:
      break;
    }
  }

  XRecordFreeData(d);
}

int watch_input() {
  /* Initialize and start Xrecord context */

  XRecordRange *rr;
  XRecordClientSpec rcs;

  dpy = XOpenDisplay(NULL);
  if (dpy == NULL) {
    fprintf(stderr, "Unable to open display\n");
    return -1;
  }

  rr = XRecordAllocRange();
  if (rr == NULL) {
    fprintf(stderr, "XRecordAllocRange error\n");
    return -1;
  }

  rr->device_events.first = KeyPress;
  rr->device_events.last = ButtonReleaseMask;
  rcs = XRecordAllClients;

  rc = XRecordCreateContext(dpy, 0, &rcs, 1, &rr, 1);
  if (rc == 0) {
    fprintf(stderr, "XRecordCreateContext error\n");
    return -1;
  }

  XFree(rr);

  if (XRecordEnableContext(dpy, rc, key_pressed_cb, NULL) == 0) {
    fprintf(stderr, "XRecordEnableContext error\n");
    return -1;
  }

  // FIXME: we want execution to resume here
  XRecordFreeContext(dpy, rc);

  return 0;
}

int main() {
  ALCdevice *device;
  ALCcontext *context;
  ALshort data[BUFFER_LENGTH * 2];

  // Initialization
  device = alcOpenDevice(NULL);
  context = alcCreateContext(device, NULL);
  alcMakeContextCurrent(context);

  double a = pow(2.0, 1.0 / 12.0);
  for (int note = 0; note < NOTES; note++) {
    // Generate sine wave data
    for (int i = 0; i < BUFFER_LENGTH; ++i) {
      double freq = 55.0 * pow(a, (double)note);
      data[i * 2] = sin(2 * M_PI * freq * i / BUFFER_LENGTH) * SHRT_MAX;
      data[i * 2 + 1] =
          -1 * sin(2 * M_PI * freq * i / BUFFER_LENGTH) * SHRT_MAX; // antiphase
    }

    // Output looping sine wave
    alGenBuffers(1, &buf[note]);
    alBufferData(buf[note], AL_FORMAT_STEREO16, data, sizeof(data),
                 BUFFER_LENGTH * 2);
    alGenSources(1, &src[note]);
    alSourcei(src[note], AL_BUFFER, buf[note]);
    alSourcei(src[note], AL_LOOPING, AL_TRUE);
  }

  printf("before\n");
  watch_input();
  printf("after\n");

  for (int note = 0; note < NOTES; note++) {
    alDeleteSources(1, &src[note]);
    alDeleteBuffers(1, &buf[note]);
  }

  alcMakeContextCurrent(NULL);
  alcDestroyContext(context);
  alcCloseDevice(device);

  return 0;
}
