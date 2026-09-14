#include <backbeat.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static int valid_asset_id(const char *value) {
  if (strlen(value) != 64) {
    return 0;
  }
  for (size_t index = 0; index < 64; index++) {
    char byte = value[index];
    if (!((byte >= '0' && byte <= '9') || (byte >= 'a' && byte <= 'f'))) {
      return 0;
    }
  }
  return 1;
}

static int write_file(const char *path) {
  FILE *file = fopen(path, "rb");
  if (file == NULL) {
    perror(path);
    return 0;
  }

  unsigned char buffer[16384];
  size_t size;
  while ((size = fread(buffer, 1, sizeof(buffer), file)) != 0) {
    if (fwrite(buffer, 1, size, stdout) != size) {
      fclose(file);
      return 0;
    }
  }

  int ok = !ferror(file);
  fclose(file);
  return ok;
}

int main(int argc, char **argv) {
  if (argc != 2) {
    fprintf(stderr, "usage: %s ASSET_ID\n", argv[0]);
    return EXIT_FAILURE;
  }
  if (!valid_asset_id(argv[1])) {
    fprintf(stderr,
            "ASSET_ID must be exactly 64 lowercase hexadecimal characters; "
            "got %zu\n",
            strlen(argv[1]));
    return EXIT_FAILURE;
  }

  bkb_store *store = NULL;
  bkb_error_code code = bkb_store_open(&store);
  if (code != BKB_OK) {
    fprintf(stderr, "bkb_store_open: %s (%d)\n", bkb_error_string(code),
            code);
    return EXIT_FAILURE;
  }

  bkb_asset_data data;
  code = bkb_store_get_asset(store, argv[1], &data);
  if (code != BKB_OK) {
    fprintf(stderr, "bkb_store_get_asset: %s (%d)\n",
            bkb_error_string(code), code);
    bkb_store_free(store);
    return EXIT_FAILURE;
  }

  int ok;
  switch (data.kind) {
  case BKB_ASSET_DATA_BYTES:
    fprintf(stderr, "asset kind: bytes (%zu bytes)\n", data.value.bytes.len);
    ok = fwrite(data.value.bytes.ptr, 1, data.value.bytes.len, stdout) ==
         data.value.bytes.len;
    break;
  case BKB_ASSET_DATA_FILE:
    fprintf(stderr, "asset kind: file (%s)\n", data.value.file.ptr);
    ok = write_file(data.value.file.ptr);
    break;
  default:
    fprintf(stderr, "unknown asset data kind: %d\n", data.kind);
    ok = 0;
    break;
  }

  bkb_asset_data_free(data);
  bkb_store_free(store);
  return ok ? EXIT_SUCCESS : EXIT_FAILURE;
}
