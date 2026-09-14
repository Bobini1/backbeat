#include <backbeat.h>
#include <stdio.h>
#include <stdlib.h>

typedef bkb_error_code (*bkb_string_getter)(const bkb_store *, bkb_string *);

static int check(const char *operation, bkb_error_code code) {
  if (code == BKB_OK) {
    return 1;
  }
  fprintf(stderr, "%s: %s (%d)\n", operation, bkb_error_string(code), code);
  return 0;
}

static bkb_error_code print_string(const char *label, const bkb_store *store,
                                   bkb_string_getter getter) {
  bkb_string value;
  bkb_error_code code = getter(store, &value);
  if (code != BKB_OK) {
    return code;
  }
  printf("%s: ", label);
  if (value.len != 0) {
    fwrite(value.ptr, 1, value.len, stdout);
  }
  fputc('\n', stdout);
  bkb_string_free(value);
  return BKB_OK;
}

int main(void) {
  bkb_store *store = NULL;
  bkb_error_code code = bkb_store_open(&store);
  if (!check("bkb_store_open", code)) {
    return EXIT_FAILURE;
  }

  int status = EXIT_FAILURE;

  code = print_string("store", store, bkb_store_dir);
  if (!check("bkb_store_dir", code)) {
    goto done;
  }

  code = print_string("logs", store, bkb_store_logs_dir);
  if (!check("bkb_store_logs_dir", code)) {
    goto done;
  }

  code = print_string("config", store, bkb_store_config_dir);
  if (!check("bkb_store_config_dir", code)) {
    goto done;
  }

  code = print_string("sqlite URL", store, bkb_store_sqlite_connection_url);
  if (!check("bkb_store_sqlite_connection_url", code)) {
    goto done;
  }

  code = print_string("attach SQL", store, bkb_store_sqlite_attach_command);
  if (!check("bkb_store_sqlite_attach_command", code)) {
    goto done;
  }

  code = print_string("detach SQL", store, bkb_store_sqlite_detach_command);
  if (!check("bkb_store_sqlite_detach_command", code)) {
    goto done;
  }

  bool has_zero_data_servers;
  code = bkb_store_has_zero_data_servers(store, &has_zero_data_servers);
  if (!check("bkb_store_has_zero_data_servers", code)) {
    goto done;
  }
  printf("data servers configured: %s\n", has_zero_data_servers ? "no" : "yes");

  status = EXIT_SUCCESS;

done:
  bkb_store_free(store);
  return status;
}
