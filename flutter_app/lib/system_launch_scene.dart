import 'dart:io';
import 'dart:math' as math;

import 'package:flutter/material.dart';

/// Présentation uniquement : la résolution de ROM et le lancement restent dans Rust.
enum LaunchMedium { cartridge, disc, handheld, arcade, computer }

class LaunchIdentity {
  const LaunchIdentity(
    this.label,
    this.accent,
    this.medium, {
    this.isMegaDrive = false,
  });

  final String label;
  final Color accent;
  final LaunchMedium medium;
  final bool isMegaDrive;

  static LaunchIdentity forSystem(String name) {
    final key = name.toLowerCase().replaceAll(RegExp(r'[^a-z0-9]'), '');
    // Priorité aux variantes : Game Boy Color avant Game Boy, PS2 avant PS1.
    if (key.contains('megadrive') || key.contains('genesis')) {
      return const LaunchIdentity(
        'Mega Drive',
        Color(0xFFE94C4C),
        LaunchMedium.cartridge,
        isMegaDrive: true,
      );
    }
    if (key.contains('gameboyadvance') || key == 'gba') {
      return const LaunchIdentity(
        'Game Boy Advance',
        Color(0xFF9280EA),
        LaunchMedium.handheld,
      );
    }
    if (key.contains('gameboycolor') || key == 'gbc') {
      return const LaunchIdentity(
        'Game Boy Color',
        Color(0xFFF5BD4B),
        LaunchMedium.handheld,
      );
    }
    if (key.contains('gameboy') || key == 'gb') {
      return const LaunchIdentity(
        'Game Boy',
        Color(0xFF93AC76),
        LaunchMedium.handheld,
      );
    }
    if (key.contains('playstation2') || key == 'ps2') {
      return const LaunchIdentity(
        'PlayStation 2',
        Color(0xFF657FEC),
        LaunchMedium.disc,
      );
    }
    if (key.contains('psp') || key.contains('playstationportable')) {
      return const LaunchIdentity(
        'PSP',
        Color(0xFFB2B7D2),
        LaunchMedium.handheld,
      );
    }
    if (key.contains('playstation') || key == 'ps1' || key == 'psx') {
      return const LaunchIdentity(
        'PlayStation',
        Color(0xFFDA6668),
        LaunchMedium.disc,
      );
    }
    if (key.contains('gamegear')) {
      return const LaunchIdentity(
        'Game Gear',
        Color(0xFFEF7075),
        LaunchMedium.handheld,
      );
    }
    if (key.contains('lynx')) {
      return const LaunchIdentity(
        'Lynx',
        Color(0xFFF1AE67),
        LaunchMedium.handheld,
      );
    }
    if (key.contains('gamecube')) {
      return const LaunchIdentity(
        'GameCube',
        Color(0xFF947AE9),
        LaunchMedium.disc,
      );
    }
    if (key.contains('nintendo64') || key == 'n64') {
      return const LaunchIdentity(
        'Nintendo 64',
        Color(0xFF61B791),
        LaunchMedium.cartridge,
      );
    }
    if (key.contains('supernintendo') ||
        key.contains('snes') ||
        key.contains('superfamicom')) {
      return const LaunchIdentity(
        'Super Nintendo',
        Color(0xFFAF96E6),
        LaunchMedium.cartridge,
      );
    }
    if (key.contains('nintendoentertainment') ||
        key == 'nes' ||
        key == 'famicom') {
      return const LaunchIdentity(
        'NES',
        Color(0xFFE57C6B),
        LaunchMedium.cartridge,
      );
    }
    if (key.contains('mastersystem')) {
      return const LaunchIdentity(
        'Master System',
        Color(0xFFDD6D71),
        LaunchMedium.cartridge,
      );
    }
    if (key.contains('dreamcast')) {
      return const LaunchIdentity(
        'Dreamcast',
        Color(0xFFFF914B),
        LaunchMedium.disc,
      );
    }
    if (key.contains('saturn')) {
      return const LaunchIdentity(
        'Saturn',
        Color(0xFF8BCBD8),
        LaunchMedium.disc,
      );
    }
    if (key.contains('finalburn') || key == 'fba' || key == 'fbneo') {
      return const LaunchIdentity(
        'FinalBurn Alpha',
        Color(0xFFFF708F),
        LaunchMedium.arcade,
      );
    }
    if (key.contains('mame') || key.contains('arcade')) {
      return const LaunchIdentity(
        'MAME',
        Color(0xFFF8BE65),
        LaunchMedium.arcade,
      );
    }
    if (key == 'pc' || key.contains('windows') || key.contains('gog')) {
      return const LaunchIdentity(
        'PC',
        Color(0xFF69D1CB),
        LaunchMedium.computer,
      );
    }
    return LaunchIdentity(
      name.trim().isEmpty ? 'Système inconnu' : name.trim(),
      const Color(0xFF67D6DA),
      LaunchMedium.cartridge,
    );
  }
}

class SystemLaunchScene extends StatefulWidget {
  const SystemLaunchScene({
    super.key,
    required this.systemName,
    required this.title,
    required this.coverUri,
    required this.onFinished,
  });

  static const duration = Duration(milliseconds: 2200);
  final String systemName;
  final String title;
  final String? coverUri;
  final VoidCallback onFinished;

  @override
  State<SystemLaunchScene> createState() => _SystemLaunchSceneState();
}

class _SystemLaunchSceneState extends State<SystemLaunchScene>
    with SingleTickerProviderStateMixin {
  late final AnimationController _controller =
      AnimationController(
        duration: SystemLaunchScene.duration,
        vsync: this,
      )..addStatusListener((status) {
        if (status == AnimationStatus.completed && mounted) widget.onFinished();
      });

  @override
  void initState() {
    super.initState();
    _controller.forward();
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final identity = LaunchIdentity.forSystem(widget.systemName);
    return Semantics(
      label: 'Lancement de ${widget.title} sur ${identity.label}',
      child: ColoredBox(
        key: const Key('system-launch-scene'),
        color: const Color(0xF9080B13),
        child: Center(
          child: FittedBox(
            fit: BoxFit.scaleDown,
            child: SizedBox(
              width: 360,
              height: 440,
              child: AnimatedBuilder(
                animation: _controller,
                builder: (context, _) {
                  final progress = _controller.value;
                  final entry = Curves.easeOutCubic.transform(
                    (progress / 0.73).clamp(0.0, 1.0),
                  );
                  final power = ((progress - 0.66) / 0.2).clamp(0.0, 1.0);
                  return Stack(
                    alignment: Alignment.center,
                    children: [
                      Positioned(
                        top: 32,
                        left: 12,
                        right: 12,
                        child: Text(
                          identity.label.toUpperCase(),
                          textAlign: TextAlign.center,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: TextStyle(
                            color: identity.accent,
                            fontSize: 17,
                            fontWeight: FontWeight.w900,
                            letterSpacing: 3,
                          ),
                        ),
                      ),
                      Positioned(
                        top: 93,
                        left: 30,
                        child: CustomPaint(
                          size: const Size(300, 260),
                          painter: _HardwarePainter(
                            identity.medium,
                            identity.accent,
                            power,
                          ),
                        ),
                      ),
                      Positioned(
                        top: 78 + (1 - entry) * 54,
                        left: 117,
                        child: Opacity(
                          opacity: 0.3 + 0.7 * entry,
                          child: Transform.scale(
                            scale: 0.85 + 0.15 * entry,
                            child: _MediaBadge(
                              medium: identity.medium,
                              accent: identity.accent,
                              coverUri: widget.coverUri,
                            ),
                          ),
                        ),
                      ),
                      Positioned(
                        left: 24,
                        right: 24,
                        bottom: 26,
                        child: Column(
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            Text(
                              widget.title,
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                              style: const TextStyle(
                                color: Colors.white,
                                fontSize: 18,
                                fontWeight: FontWeight.bold,
                              ),
                            ),
                            const SizedBox(height: 8),
                            Text(
                              power == 0 ? 'PRÉPARATION' : 'DÉMARRAGE',
                              style: TextStyle(
                                color: identity.accent,
                                fontSize: 11,
                                letterSpacing: 2.5,
                                fontWeight: FontWeight.w700,
                              ),
                            ),
                          ],
                        ),
                      ),
                    ],
                  );
                },
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _MediaBadge extends StatelessWidget {
  const _MediaBadge({
    required this.medium,
    required this.accent,
    required this.coverUri,
  });
  final LaunchMedium medium;
  final Color accent;
  final String? coverUri;

  @override
  Widget build(BuildContext context) {
    Widget art = const Icon(
      Icons.videogame_asset_outlined,
      color: Colors.white70,
      size: 42,
    );
    final uri = coverUri;
    if (uri != null && uri.isNotEmpty) {
      Widget error(BuildContext context, Object error, StackTrace? stack) =>
          const Icon(Icons.image_not_supported_outlined, color: Colors.white70);
      if (uri.startsWith('http://') || uri.startsWith('https://')) {
        art = Image.network(uri, fit: BoxFit.contain, errorBuilder: error);
      } else if (uri.startsWith('file://')) {
        art = Image.file(
          File.fromUri(Uri.parse(uri)),
          fit: BoxFit.contain,
          errorBuilder: error,
        );
      }
    }
    return Container(
      key: const Key('system-launch-media'),
      width: 126,
      height: 126,
      padding: const EdgeInsets.all(15),
      decoration: BoxDecoration(
        color: const Color(0xFF202634),
        shape: medium == LaunchMedium.disc
            ? BoxShape.circle
            : BoxShape.rectangle,
        border: Border.all(color: accent, width: 3),
        boxShadow: [
          BoxShadow(color: accent.withValues(alpha: 0.35), blurRadius: 25),
        ],
      ),
      child: ClipRect(child: Center(child: art)),
    );
  }
}

class _HardwarePainter extends CustomPainter {
  const _HardwarePainter(this.medium, this.accent, this.power);
  final LaunchMedium medium;
  final Color accent;
  final double power;

  @override
  void paint(Canvas canvas, Size size) {
    final shell = Paint()..color = const Color(0xFF343A49);
    final panel = Paint()..color = const Color(0xFF121722);
    final lit = Paint()
      ..color = Color.lerp(const Color(0xFF414758), accent, power)!;
    if (medium == LaunchMedium.handheld) {
      canvas.drawRRect(
        RRect.fromRectAndRadius(
          const Rect.fromLTWH(42, 6, 216, 242),
          const Radius.circular(28),
        ),
        shell,
      );
      canvas.drawRRect(
        RRect.fromRectAndRadius(
          const Rect.fromLTWH(68, 40, 164, 137),
          const Radius.circular(8),
        ),
        panel,
      );
      canvas.drawRRect(
        RRect.fromRectAndRadius(
          const Rect.fromLTWH(76, 48, 148, 121),
          const Radius.circular(3),
        ),
        lit,
      );
      canvas.drawCircle(const Offset(93, 206), 13, panel);
      canvas.drawCircle(const Offset(210, 204), 10, lit);
    } else if (medium == LaunchMedium.arcade) {
      final cabinet = Path()
        ..moveTo(72, 9)
        ..lineTo(228, 9)
        ..lineTo(252, 244)
        ..lineTo(48, 244)
        ..close();
      canvas.drawPath(cabinet, shell);
      canvas.drawRect(const Rect.fromLTWH(80, 39, 140, 118), panel);
      canvas.drawRect(const Rect.fromLTWH(87, 46, 126, 104), lit);
      canvas.drawOval(const Rect.fromLTWH(75, 183, 150, 37), panel);
      canvas.drawCircle(const Offset(197, 200), 8, lit);
    } else if (medium == LaunchMedium.computer) {
      canvas.drawRRect(
        RRect.fromRectAndRadius(
          const Rect.fromLTWH(37, 14, 226, 171),
          const Radius.circular(10),
        ),
        shell,
      );
      canvas.drawRect(const Rect.fromLTWH(49, 27, 202, 143), panel);
      canvas.drawRect(const Rect.fromLTWH(55, 33, 190, 131), lit);
      canvas.drawRRect(
        RRect.fromRectAndRadius(
          const Rect.fromLTWH(15, 193, 270, 38),
          const Radius.circular(9),
        ),
        shell,
      );
    } else {
      canvas.drawOval(const Rect.fromLTWH(29, 106, 242, 123), shell);
      canvas.drawOval(const Rect.fromLTWH(57, 132, 186, 69), panel);
      if (medium == LaunchMedium.disc) {
        canvas.drawCircle(const Offset(150, 166), 31, lit);
        canvas.drawCircle(const Offset(150, 166), 8, panel);
      } else {
        canvas.drawRRect(
          RRect.fromRectAndRadius(
            const Rect.fromLTWH(100, 157, 100, 16),
            const Radius.circular(4),
          ),
          lit,
        );
      }
    }
    // Témoin d'alimentation, visible sur toutes les silhouettes.
    canvas.drawCircle(
      const Offset(253, 220),
      5 + 2 * math.sin(power * math.pi / 2),
      lit,
    );
  }

  @override
  bool shouldRepaint(covariant _HardwarePainter oldDelegate) =>
      oldDelegate.medium != medium ||
      oldDelegate.accent != accent ||
      oldDelegate.power != power;
}
