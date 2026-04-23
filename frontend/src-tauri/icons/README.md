# App Icons

This directory should contain the application icons for different platforms:

- `32x32.png` - Small icon (Windows taskbar, etc.)
- `128x128.png` - Standard icon
- `128x128@2x.png` - Retina/high-DPI icon
- `icon.icns` - macOS app icon
- `icon.ico` - Windows app icon

## Generating Icons

You can generate icons from a base image using online tools:
- https://www.appicon.co/ (supports all formats)
- https://icoconvert.com/ (ICO converter)
- ImageMagick: `convert image.png -resize 32x32 32x32.png`

## macOS Icon (.icns)

Use Xcode tools:
```bash
# From a 1024x1024 PNG
mkdir -p icon.iconset
cp icon.png icon.iconset/icon_1024x1024.png
iconutil -c icns icon.iconset
```

Or use online converter and download the .icns file.

## Windows Icon (.ico)

Convert from PNG using online tools or ImageMagick:
```bash
convert icon.png icon.ico
```

## Minimum Requirements

All icon files must:
1. Be valid PNG/ICO/ICNS format for their type
2. Have proper dimensions (see filenames)
3. Be placed in this directory

The build will fail if these icons are missing, so you can:
- Use placeholder icons for development
- Generate proper branded icons before release
