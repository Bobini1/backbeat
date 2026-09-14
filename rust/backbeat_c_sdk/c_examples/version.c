#include <backbeat.h>

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int main(void) {
  if (BKB_LIBVERSION != bkb_libversion()) {
    fprintf(stderr, "header library version %d does not match archive version %d\n",
            BKB_LIBVERSION, bkb_libversion());
    return EXIT_FAILURE;
  }
  if (strcmp(BKB_LIBCOMMIT_HASH, bkb_libcommithash()) != 0) {
    fprintf(stderr,
            "header commit hash %s does not match archive commit hash %s\n",
            BKB_LIBCOMMIT_HASH, bkb_libcommithash());
    return EXIT_FAILURE;
  }
  printf("Backbeat library version: %d\nCommit hash: %s\n", bkb_libversion(),
         bkb_libcommithash());
  return EXIT_SUCCESS;
}
