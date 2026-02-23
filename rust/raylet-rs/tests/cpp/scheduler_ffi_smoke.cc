#include <utility>

#include "ray/raylet/scheduling/rust_scheduler_ffi.h"

int main() {
  auto handle = ray::raylet::raylet_scheduler_create();
  ray::raylet::raylet_scheduler_update_cluster_view(*handle);
  ray::raylet::raylet_scheduler_allocate(*handle);
  ray::raylet::raylet_scheduler_release(*handle);
  ray::raylet::raylet_scheduler_destroy(std::move(handle));

  return 0;
}
