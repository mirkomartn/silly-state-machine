#include <stdio.h>
#include <stdbool.h>
#include <readline/readline.h>
#include <readline/history.h>

// Compile with: gcc -o ffi_test ffi_test.c target/x86_64-unknown-none/debug/libciao.a -lreadline
// You might need to install libreadline-dev package or similar.

extern void init(void);
extern bool step(void);

static bool received = false;
static bool pressed = false;

void got_mail() {
  puts("Got mail! It's a good day today.");
}

bool msg_received() {
  if (received == true) {
    received = false;
    return true;
  }

  return false;
}

bool button_pressed() {
  return pressed;
}

int main() {
  init();
  printf("When prompted for an action, you can do nothing or:\n\t[P]ress a button.\n\t[R]elease a pressed button.\n\tSend a [m]essage.\n\n");

  for(int i=0;;i++) {
    char *ans = readline("action [pP/rR/mM]: ");
    if (ans == NULL) {
        fprintf(stderr, "Error encountered trying to retrieve your answer.");
        return -1;
    }

    switch (ans[0]) {
      case 'P':
      case 'p':
        puts("Pressed a button.");
        pressed = true;
        break;
      case 'R':
      case 'r':
        puts("Released a button.");
        pressed = false;
        break;
      case 'M':
      case 'm':
        received = true;
        break;
      default:
        // puts("Unknown option specified please input one of 'P/R/M'.");
        break;
    }

    printf("[%d] Calling step().\n", i);
    if (step() == true) {
      puts("State machine stopped executing.");
      break;
    }
  }
}
