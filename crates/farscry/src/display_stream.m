// display_stream.m — ScreenCaptureKit wrapper (macOS 12.3+, required on macOS 15+).
//
// SCStream delivers frames at the GPU's compositor output.  We configure
// width=32, height=32 so the GPU scales BEFORE delivering to us.  The
// CVPixelBuffer in each callback is 32×32 BGRA (128–256 bytes per row).
// We lock it, call the Rust callback (which samples 1024 pixels), then unlock.
// The pixel data never reaches the Rust heap as a large allocation.
//
// Memory model (per frame):
//   CVPixelBuffer lock:  shared GPU memory → zero RSS impact
//   Rust sampling:       reads 4 KB (1024 pixels × 4 bytes) in place
//   pHash internals:     ~12 KB alloc + free (DCT on 32×32)
//   Net heap growth:     ~0 bytes (tiny allocs reused by allocator)

#import <ScreenCaptureKit/ScreenCaptureKit.h>
#import <CoreVideo/CoreVideo.h>
#import <CoreMedia/CoreMedia.h>
#import <Foundation/Foundation.h>
#import <dispatch/dispatch.h>
#include <stdlib.h>
#include <time.h>
#include <stdint.h>
#include <string.h>

// Callback invoked while the pixel buffer is locked.
// base: pointer to BGRA pixel data  bpr: bytes per row  ctx: Rust closure
typedef void (*FarscryFrameCb)(const void *base, size_t bpr, void *ctx);

typedef struct {
    FarscryFrameCb callback;
    void          *user_ctx;
    uint64_t       min_interval_ns;
    uint64_t       last_ns;
} FrameState;

typedef struct {
    SCStream   *stream;
    id          output;    // FarscryOutput (opaque to C callers)
    FrameState *state;
} StreamBundle;

static uint64_t mono_ns(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC_RAW, &ts);
    return (uint64_t)ts.tv_sec * 1000000000ULL + (uint64_t)ts.tv_nsec;
}

// ─── SCStreamOutput delegate ──────────────────────────────────────────────────

@interface FarscryOutput : NSObject <SCStreamOutput, SCStreamDelegate>
@property (nonatomic, assign) FrameState *state;
@end

@implementation FarscryOutput

- (void)stream:(SCStream *)stream
     didOutputSampleBuffer:(CMSampleBufferRef)sampleBuffer
               ofType:(SCStreamOutputType)type
{
    if (type != SCStreamOutputTypeScreen) return;

    uint64_t now = mono_ns();
    if (now - self.state->last_ns < self.state->min_interval_ns) return;
    self.state->last_ns = now;

    CVPixelBufferRef pix = CMSampleBufferGetImageBuffer(sampleBuffer);
    if (!pix) return;

    if (CVPixelBufferLockBaseAddress(pix, kCVPixelBufferLock_ReadOnly) != kCVReturnSuccess) return;

    const void *base = CVPixelBufferGetBaseAddress(pix);
    size_t      bpr  = CVPixelBufferGetBytesPerRow(pix);

    if (base) {
        self.state->callback(base, bpr, self.state->user_ctx);
    }

    CVPixelBufferUnlockBaseAddress(pix, kCVPixelBufferLock_ReadOnly);
}

- (void)stream:(SCStream *)stream didStopWithError:(NSError *)error {
    (void)stream;
    (void)error;
}

@end

// ─── Public C API ─────────────────────────────────────────────────────────────

// Start an SCStream on the primary display, scaled to (out_w × out_h).
// Returns NULL on failure (e.g. no Screen Recording permission).
void *farscry_stream_start(
    uint32_t      display_id __attribute__((unused)),
    size_t        out_w,
    size_t        out_h,
    uint32_t      fps_limit,
    FarscryFrameCb callback,
    void         *ctx
) {
    if (!callback) return NULL;

    FrameState *state = calloc(1, sizeof(FrameState));
    state->callback       = callback;
    state->user_ctx       = ctx;
    state->min_interval_ns = fps_limit > 0
        ? 1000000000ULL / fps_limit
        : 500000000ULL;   // default: 2 FPS max

    dispatch_semaphore_t sem = dispatch_semaphore_create(0);
    __block StreamBundle *result = NULL;

    [SCShareableContent
        getShareableContentExcludingDesktopWindows:NO
        onScreenWindowsOnly:NO
        completionHandler:^(SCShareableContent *content, NSError *err) {

        if (err || !content || !content.displays.count) {
            dispatch_semaphore_signal(sem);
            return;
        }

        SCDisplay *display = content.displays[0];
        SCContentFilter *filter =
            [[SCContentFilter alloc] initWithDisplay:display
                                   excludingWindows:@[]];

        SCStreamConfiguration *config = [[SCStreamConfiguration alloc] init];
        config.width           = out_w;
        config.height          = out_h;
        config.pixelFormat     = kCVPixelFormatType_32BGRA;
        config.capturesAudio   = NO;
        config.showsCursor     = NO;
        config.minimumFrameInterval = CMTimeMake(1, (int32_t)(fps_limit > 0 ? fps_limit : 2));

        FarscryOutput *output = [[FarscryOutput alloc] init];
        output.state = state;

        SCStream *stream = [[SCStream alloc] initWithFilter:filter
                                             configuration:config
                                                  delegate:output];

        NSError *addErr = nil;
        BOOL added = [stream addStreamOutput:output
                                        type:SCStreamOutputTypeScreen
                          sampleHandlerQueue:dispatch_get_global_queue(QOS_CLASS_UTILITY, 0)
                                       error:&addErr];
        if (!added || addErr) {
            dispatch_semaphore_signal(sem);
            return;
        }

        [stream startCaptureWithCompletionHandler:^(NSError *startErr) {
            if (!startErr) {
                StreamBundle *b  = calloc(1, sizeof(StreamBundle));
                b->stream     = stream;
                b->output     = output;
                b->state      = state;
                result = b;
            }
            dispatch_semaphore_signal(sem);
        }];
    }];

    dispatch_semaphore_wait(sem, dispatch_time(DISPATCH_TIME_NOW, 6LL * NSEC_PER_SEC));

    if (!result) {
        free(state);
        return NULL;
    }
    return result;
}

// Stop and free the stream.
void farscry_stream_stop(void *handle) {
    if (!handle) return;
    StreamBundle *b = (StreamBundle *)handle;
    dispatch_semaphore_t sem = dispatch_semaphore_create(0);
    [b->stream stopCaptureWithCompletionHandler:^(NSError *e) {
        (void)e;
        dispatch_semaphore_signal(sem);
    }];
    dispatch_semaphore_wait(sem, dispatch_time(DISPATCH_TIME_NOW, 2LL * NSEC_PER_SEC));
    free(b->state);
    free(b);
}

// Stubs so the IOSurface helpers in iosurface_phash.rs compile even though
// we no longer use them from the stream path.
int    farscry_surface_lock(void *s)      { (void)s; return 0; }
void   farscry_surface_unlock(void *s)    { (void)s; }
void  *farscry_surface_base(void *s)      { (void)s; return NULL; }
size_t farscry_surface_bpr(void *s)       { (void)s; return 0; }
size_t farscry_surface_width(void *s)     { (void)s; return 0; }
size_t farscry_surface_height(void *s)    { (void)s; return 0; }
