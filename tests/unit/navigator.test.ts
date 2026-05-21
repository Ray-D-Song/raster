describe("navigator.userAgent", () => {
  it('should start with "raster_runtime "', () => {
    expect(navigator.userAgent.startsWith("raster_runtime ")).toBeTruthy();
  });
});
