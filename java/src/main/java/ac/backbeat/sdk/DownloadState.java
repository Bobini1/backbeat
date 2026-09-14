package ac.backbeat.sdk;

public enum DownloadState {
    QUEUED(NativeBindings.DOWNLOAD_QUEUED),
    DOWNLOADING(NativeBindings.DOWNLOAD_DOWNLOADING),
    VERIFYING(NativeBindings.DOWNLOAD_VERIFYING),
    COMMITTING(NativeBindings.DOWNLOAD_COMMITTING),
    DONE(NativeBindings.DOWNLOAD_DONE),
    FAILED(NativeBindings.DOWNLOAD_FAILED),
    CANCELLED(NativeBindings.DOWNLOAD_CANCELLED);

    private final int nativeValue;

    DownloadState(int nativeValue) { this.nativeValue = nativeValue; }

    static DownloadState fromNative(int value) {
        for (DownloadState state : values()) if (state.nativeValue == value) return state;
        throw new BackbeatException("Unknown Backbeat download state: " + value, null);
    }
}
