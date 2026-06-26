import Foundation

public typealias RasterPluginHandler = @convention(c) (UnsafeRawPointer?) -> Void

public struct RasterPluginCall {
    public var call_id: UInt64
    public var plugin: UnsafePointer<CChar>?
    public var method: UnsafePointer<CChar>?
    public var args_json: UnsafePointer<CChar>
    public var context: UnsafeMutableRawPointer?
}

@_silgen_name("raster_plugin_register_method")
public func raster_plugin_register_method(
    _ plugin: UnsafePointer<CChar>?,
    _ method: UnsafePointer<CChar>?,
    _ handler: RasterPluginHandler?,
    _ context: UnsafeMutableRawPointer?
) -> Bool

@_silgen_name("raster_plugin_reply_ok")
public func raster_plugin_reply_ok(_ callId: UInt64, _ resultJson: UnsafePointer<CChar>?)

@_silgen_name("raster_plugin_reply_err")
public func raster_plugin_reply_err(
    _ callId: UInt64,
    _ code: UnsafePointer<CChar>?,
    _ message: UnsafePointer<CChar>?
)

@_silgen_name("raster_plugin_emit_event")
public func raster_plugin_emit_event(
    _ plugin: UnsafePointer<CChar>?,
    _ event: UnsafePointer<CChar>?,
    _ dataJson: UnsafePointer<CChar>?
)

@_silgen_name("raster_ios_host_view_controller")
public func raster_ios_host_view_controller() -> UnsafeMutableRawPointer?

@_silgen_name("raster_ios_set_host_view_controller")
public func raster_ios_set_host_view_controller(_ viewController: UnsafeMutableRawPointer?)