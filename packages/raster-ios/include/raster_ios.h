#ifndef RASTER_IOS_H
#define RASTER_IOS_H

#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

bool raster_ios_run_app(const char *bundle_name, const char *bundle_source, const char *dev_config_json);
const char *raster_ios_last_error(void);
void raster_ios_set_host_view_controller(void *view_controller);
void *raster_ios_host_view_controller(void);
void *raster_ios_root_view(void);
void raster_ios_request_frame(void);
void raster_ios_will_enter_foreground(void);
void raster_ios_did_become_active(void);
void raster_ios_will_resign_active(void);
void raster_ios_did_enter_background(void);
void raster_ios_will_terminate(void);
void raster_ios_handle_open_url(void *url_ptr);

#ifdef __cplusplus
}
#endif

#endif
