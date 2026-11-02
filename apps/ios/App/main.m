#import <Foundation/Foundation.h>
extern int vp_client_ios_main(void);
int main(int argc, char *argv[]) {
    @autoreleasepool {
        // eframe/winit creates UIApplication and runs its event loop.
        return vp_client_ios_main();
    }
}
