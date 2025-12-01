#include <stdio.h>
#include <stdbool.h>

extern void init(void);
extern bool step(void);

int main() {
  int steps = 20;
  init();

  for (int i = 1; i < steps; i++) {
    printf("[%d/%d] Calling step().\n", i, steps);
    if (step() == true) {
      break;
    }
  }
}
