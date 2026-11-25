// Thin C ABI for Apple's classic Bluetooth and audio APIs. Link objects and
// their delegates are created, pumped, and released on one Rust-owned thread.
#import <Foundation/Foundation.h>
#import <IOBluetooth/IOBluetooth.h>
#import <CoreAudio/CoreAudio.h>

char *qc_paired_devices(void) {
    @autoreleasepool {
        NSMutableArray *items = [NSMutableArray array];
        for (IOBluetoothDevice *device in [IOBluetoothDevice pairedDevices]) {
            IOBluetoothSDPServiceRecord *record = [device getServiceRecordForUUID:
                [IOBluetoothSDPUUID uuid16:0x1200]];
            [items addObject:@{
                @"address": device.addressString ?: @"",
                @"name": device.name ?: @"",
                @"connected": @([device isConnected]),
                @"vendor": [[record getAttributeDataElement:0x0201] getNumberValue] ?: [NSNull null],
                @"source": [[record getAttributeDataElement:0x0205] getNumberValue] ?: [NSNull null]
            }];
        }
        NSData *data = [NSJSONSerialization dataWithJSONObject:items options:0 error:nil];
        return strdup([[NSString alloc] initWithData:data encoding:NSUTF8StringEncoding].UTF8String ?: "[]");
    }
}

char *qc_audio_outputs(void) {
    @autoreleasepool {
        NSMutableArray *names = [NSMutableArray array];
        AudioObjectPropertyAddress prop = {kAudioHardwarePropertyDevices,
            kAudioObjectPropertyScopeGlobal, kAudioObjectPropertyElementMain};
        UInt32 size = 0;
        if (AudioObjectGetPropertyDataSize(kAudioObjectSystemObject, &prop, 0, NULL, &size) == noErr && size) {
            AudioDeviceID *devices = malloc(size);
            if (AudioObjectGetPropertyData(kAudioObjectSystemObject, &prop, 0, NULL, &size, devices) == noErr) {
                for (UInt32 i = 0; i < size / sizeof(AudioDeviceID); i++) {
                    AudioObjectPropertyAddress streams = {kAudioDevicePropertyStreamConfiguration,
                        kAudioObjectPropertyScopeOutput, kAudioObjectPropertyElementMain};
                    UInt32 bytes = 0;
                    if (AudioObjectGetPropertyDataSize(devices[i], &streams, 0, NULL, &bytes) != noErr || !bytes) continue;
                    AudioBufferList *list = malloc(bytes);
                    BOOL output = NO;
                    if (AudioObjectGetPropertyData(devices[i], &streams, 0, NULL, &bytes, list) == noErr) {
                        for (UInt32 j = 0; j < list->mNumberBuffers; j++) output |= list->mBuffers[j].mNumberChannels > 0;
                    }
                    free(list);
                    if (!output) continue;
                    AudioObjectPropertyAddress nameProp = {kAudioObjectPropertyName,
                        kAudioObjectPropertyScopeGlobal, kAudioObjectPropertyElementMain};
                    CFStringRef name = NULL;
                    UInt32 nameSize = sizeof(name);
                    if (AudioObjectGetPropertyData(devices[i], &nameProp, 0, NULL, &nameSize, &name) == noErr && name) {
                        [names addObject:CFBridgingRelease(name)];
                    }
                }
            }
            free(devices);
        }
        NSData *data = [NSJSONSerialization dataWithJSONObject:names options:0 error:nil];
        return strdup([[NSString alloc] initWithData:data encoding:NSUTF8StringEncoding].UTF8String ?: "[]");
    }
}
void qc_free_string(char *value) { free(value); }

@interface QCLink : NSObject <IOBluetoothRFCOMMChannelDelegate>
@property(nonatomic, strong) IOBluetoothRFCOMMChannel *channel;
@property(nonatomic, strong) NSMutableData *buffer;
@property(nonatomic) BOOL opened;
@property(nonatomic) BOOL closed;
@end
@implementation QCLink
- (void)rfcommChannelOpenComplete:(IOBluetoothRFCOMMChannel *)channel status:(IOReturn)status {
    self.opened = status == kIOReturnSuccess;
    self.closed = status != kIOReturnSuccess;
}
- (void)rfcommChannelData:(IOBluetoothRFCOMMChannel *)channel data:(void *)data length:(size_t)length {
    // Fail rather than silently truncate a response if the peer floods us.
    if (self.buffer.length + length > 256 * 1024) { self.closed = YES; return; }
    [self.buffer appendBytes:data length:length];
}
- (void)rfcommChannelClosed:(IOBluetoothRFCOMMChannel *)channel { self.closed = YES; }
@end

void qc_pump(double seconds) {
    @autoreleasepool {
        // CFRunLoopRunInMode returns immediately with no sources, so sleep the
        // remainder to avoid spinning while disconnected.
        NSDate *end = [NSDate dateWithTimeIntervalSinceNow:seconds];
        [[NSRunLoop currentRunLoop] runMode:NSDefaultRunLoopMode beforeDate:end];
        NSTimeInterval remaining = [end timeIntervalSinceNow];
        if (remaining > 0) [NSThread sleepForTimeInterval:remaining];
    }
}

void *qc_open(const char *address, uint8_t channelID, double timeout, int *status) {
    @autoreleasepool {
        QCLink *link = [QCLink new];
        link.buffer = [NSMutableData data];
        NSString *normalized = [[NSString stringWithUTF8String:address] stringByReplacingOccurrencesOfString:@":" withString:@"-"];
        IOBluetoothDevice *device = [IOBluetoothDevice deviceWithAddressString:normalized];
        if (!device) { *status = -1; return NULL; }
        IOBluetoothRFCOMMChannel *channel = nil;
        IOReturn result = [device openRFCOMMChannelAsync:&channel withChannelID:channelID delegate:link];
        link.channel = channel;
        if (result == kIOReturnSuccess) {
            NSTimeInterval deadline = [NSProcessInfo processInfo].systemUptime + timeout;
            while (!link.opened && !link.closed && [NSProcessInfo processInfo].systemUptime < deadline) qc_pump(0.008);
        }
        if (result != kIOReturnSuccess || !link.opened || link.closed || !channel) {
            [channel setDelegate:nil];
            [channel closeChannel];
            *status = result != kIOReturnSuccess ? (int)result : (link.closed ? -1 : -2);
            return NULL;
        }
        *status = 0;
        return (__bridge_retained void *)link;
    }
}
int qc_write(void *handle, const uint8_t *data, size_t length) {
    @autoreleasepool {
        QCLink *link = (__bridge QCLink *)handle;
        if (link.closed || length > UINT16_MAX) return -1;
        [link.buffer setLength:0];
        return [link.channel writeSync:(void *)data length:(UInt16)length];
    }
}
// Returns bytes copied, or -1 after disconnect. Pump before calling.
long qc_read(void *handle, uint8_t *data, size_t capacity) {
    @autoreleasepool {
        QCLink *link = (__bridge QCLink *)handle;
        if (link.closed) return -1;
        NSUInteger count = MIN(capacity, link.buffer.length);
        memcpy(data, link.buffer.bytes, count);
        [link.buffer replaceBytesInRange:NSMakeRange(0, count) withBytes:NULL length:0];
        return (long)count;
    }
}
void qc_close(void *handle) {
    @autoreleasepool {
        QCLink *link = (__bridge_transfer QCLink *)handle;
        [link.channel setDelegate:nil];
        [link.channel closeChannel];
        link.channel = nil;
        // Do not close the device's baseband connection: macOS audio uses it.
    }
}
