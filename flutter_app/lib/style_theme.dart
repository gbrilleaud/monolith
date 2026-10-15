import 'dart:convert';
import 'dart:ui' show lerpDouble;

import 'package:flutter/material.dart';

/// Flutter presentation of the canonical Rust style snapshot (no persistence).
class StyleTheme {
  StyleTheme._(this.snapshot);

  final Map<String, dynamic> snapshot;

  factory StyleTheme.fromSnapshot(String json) =>
      StyleTheme._(jsonDecode(json) as Map<String, dynamic>);

  static final fallback = StyleTheme._({
    'selected': 'dark',
    'cartridge_animation': true,
    'catalog': {'presets': <Object>[]},
    'resolved': {
      'colors': {
        'background': '#080C15',
        'surface': '#141B29',
        'surface_raised': '#202A3C',
        'primary': '#67D6DA',
        'secondary': '#FFC578',
        'text': '#FFFFFF',
        'text_muted': '#A3ADBB',
        'border': '#3A4555',
        'error': '#FF8585',
      },
      'font_family': 'sans',
      'density': 1.0,
      'tile_spacing': 10.0,
      'corner_radius': 10.0,
    },
  });

  Map<String, dynamic> get resolved =>
      snapshot['resolved'] as Map<String, dynamic>;
  Map<String, dynamic> get colors => resolved['colors'] as Map<String, dynamic>;
  bool get cartridgeAnimation =>
      snapshot['cartridge_animation'] as bool? ?? true;
  String get selected => snapshot['selected'] as String? ?? 'dark';
  List<Map<String, dynamic>> get presets =>
      (snapshot['catalog'] as Map<String, dynamic>)['presets']
          .cast<Map<String, dynamic>>();
  String? get backgroundImage => resolved['background_image'] as String?;
  Color color(String name) =>
      Color(int.parse('FF${(colors[name] as String).substring(1)}', radix: 16));

  Color _contrasting(Color color) =>
      color.computeLuminance() > 0.4 ? Colors.black : Colors.white;

  ThemeData toThemeData({String? dataDirectory}) {
    final background = color('background');
    final surface = color('surface');
    final text = color('text');
    final primary = color('primary');
    final secondary = color('secondary');
    final radius = (resolved['corner_radius'] as num).toDouble();
    final density = (resolved['density'] as num).toDouble();
    final spacing = (resolved['tile_spacing'] as num).toDouble();
    final font = switch (resolved['font_family']) {
      'serif' => 'serif',
      'mono' => 'monospace',
      _ => 'Roboto',
    };
    final brightness = background.computeLuminance() > 0.4
        ? Brightness.light
        : Brightness.dark;
    final base = ThemeData(brightness: brightness, useMaterial3: true);
    final scheme =
        ColorScheme.fromSeed(
          seedColor: primary,
          brightness: brightness,
        ).copyWith(
          primary: primary,
          onPrimary: _contrasting(primary),
          secondary: secondary,
          onSecondary: _contrasting(secondary),
          surface: surface,
          onSurface: text,
          onSurfaceVariant: color('text_muted'),
          surfaceContainer: color('surface_raised'),
          surfaceContainerHigh: color('surface_raised'),
          surfaceContainerLow: surface,
          error: color('error'),
          outline: color('border'),
        );
    final shape = RoundedRectangleBorder(
      borderRadius: BorderRadius.circular(radius),
    );
    return base.copyWith(
      colorScheme: scheme,
      scaffoldBackgroundColor: background,
      canvasColor: surface,
      dividerColor: color('border'),
      textTheme: base.textTheme.apply(
        fontFamily: font,
        bodyColor: text,
        displayColor: text,
      ),
      visualDensity: VisualDensity(
        horizontal: (density - 1) * 4,
        vertical: (density - 1) * 4,
      ),
      cardTheme: CardThemeData(color: color('surface_raised'), shape: shape),
      dialogTheme: DialogThemeData(backgroundColor: surface, shape: shape),
      extensions: [
        StyleTokens(
          tileSpacing: spacing,
          cornerRadius: radius,
          surfaceRaised: color('surface_raised'),
          textMuted: color('text_muted'),
          backgroundImage: backgroundImage == null || dataDirectory == null
              ? null
              : '$dataDirectory/themes/backgrounds/$backgroundImage',
        ),
      ],
    );
  }
}

@immutable
class StyleTokens extends ThemeExtension<StyleTokens> {
  const StyleTokens({
    required this.tileSpacing,
    required this.cornerRadius,
    required this.surfaceRaised,
    required this.textMuted,
    this.backgroundImage,
  });
  final double tileSpacing;
  final double cornerRadius;
  final Color surfaceRaised;
  final Color textMuted;
  final String? backgroundImage;

  @override
  StyleTokens copyWith({
    double? tileSpacing,
    double? cornerRadius,
    Color? surfaceRaised,
    Color? textMuted,
    String? backgroundImage,
  }) => StyleTokens(
    tileSpacing: tileSpacing ?? this.tileSpacing,
    cornerRadius: cornerRadius ?? this.cornerRadius,
    surfaceRaised: surfaceRaised ?? this.surfaceRaised,
    textMuted: textMuted ?? this.textMuted,
    backgroundImage: backgroundImage ?? this.backgroundImage,
  );

  @override
  StyleTokens lerp(ThemeExtension<StyleTokens>? other, double t) {
    if (other is! StyleTokens) return this;
    return StyleTokens(
      tileSpacing: lerpDouble(tileSpacing, other.tileSpacing, t)!,
      cornerRadius: lerpDouble(cornerRadius, other.cornerRadius, t)!,
      surfaceRaised: Color.lerp(surfaceRaised, other.surfaceRaised, t)!,
      textMuted: Color.lerp(textMuted, other.textMuted, t)!,
      backgroundImage: t < .5 ? backgroundImage : other.backgroundImage,
    );
  }
}
