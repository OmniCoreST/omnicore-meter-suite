#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Generate all required icon sizes for Tauri from a source image.
Usage: python generate-icons.py
"""

from PIL import Image
import os
import sys

# Set UTF-8 encoding for Windows console
if sys.platform == 'win32':
    try:
        sys.stdout.reconfigure(encoding='utf-8')
    except:
        pass

# Source icon
SOURCE_ICON = "src-tauri/icons/omnicore.png"
ICONS_DIR = "src-tauri/icons"

# Required icon sizes for Tauri
ICON_SIZES = {
    # Standard Tauri icons
    "32x32.png": (32, 32),
    "128x128.png": (128, 128),
    "128x128@2x.png": (256, 256),
    "icon.png": (512, 512),

    # Windows Store icons
    "Square30x30Logo.png": (30, 30),
    "Square44x44Logo.png": (44, 44),
    "Square71x71Logo.png": (71, 71),
    "Square89x89Logo.png": (89, 89),
    "Square107x107Logo.png": (107, 107),
    "Square142x142Logo.png": (142, 142),
    "Square150x150Logo.png": (150, 150),
    "Square284x284Logo.png": (284, 284),
    "Square310x310Logo.png": (310, 310),
    "StoreLogo.png": (50, 50),
}

def generate_icons():
    """Generate all required icon sizes from source image."""

    if not os.path.exists(SOURCE_ICON):
        print(f"❌ Error: Source icon not found: {SOURCE_ICON}")
        return False

    print(f"📂 Loading source icon: {SOURCE_ICON}")

    try:
        # Open source image
        source = Image.open(SOURCE_ICON)
        print(f"✓ Source image loaded: {source.size[0]}x{source.size[1]} pixels")

        # Convert to RGBA if not already
        if source.mode != 'RGBA':
            source = source.convert('RGBA')
            print(f"✓ Converted to RGBA mode")

        # Generate each required size
        print(f"\n🎨 Generating {len(ICON_SIZES)} icon sizes...")

        for filename, (width, height) in ICON_SIZES.items():
            output_path = os.path.join(ICONS_DIR, filename)

            # Resize with high-quality resampling
            resized = source.resize((width, height), Image.Resampling.LANCZOS)

            # Save
            resized.save(output_path, "PNG", optimize=True)
            print(f"  ✓ {filename:25s} ({width}x{height})")

        # Generate ICO file with multiple sizes
        print(f"\n💾 Generating icon.ico...")
        ico_sizes = [(16, 16), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]
        ico_images = []

        for size in ico_sizes:
            ico_images.append(source.resize(size, Image.Resampling.LANCZOS))

        ico_path = os.path.join(ICONS_DIR, "icon.ico")
        ico_images[0].save(
            ico_path,
            format='ICO',
            sizes=[(img.width, img.height) for img in ico_images],
            append_images=ico_images[1:]
        )
        print(f"  ✓ icon.ico (multi-size: {', '.join([f'{s[0]}x{s[1]}' for s in ico_sizes])})")

        # Generate ICNS for macOS (if needed)
        try:
            # ICNS generation requires additional setup, skip for now
            print(f"\n⚠ Note: icon.icns (macOS) not generated - requires additional tools")
            print(f"  You can generate it manually or use an online converter")
        except:
            pass

        print(f"\n✅ Success! Generated all icon files in {ICONS_DIR}/")
        print(f"\n📝 Next steps:")
        print(f"  1. Restart your Tauri app to see the new icon")
        print(f"  2. For macOS icon (icon.icns), use: https://cloudconvert.com/png-to-icns")

        return True

    except Exception as e:
        print(f"\n❌ Error generating icons: {e}")
        import traceback
        traceback.print_exc()
        return False

if __name__ == "__main__":
    print("=" * 60)
    print("  Tauri Icon Generator")
    print("=" * 60)
    print()

    success = generate_icons()

    if success:
        print("\n" + "=" * 60)
        exit(0)
    else:
        exit(1)
