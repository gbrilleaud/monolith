import 'dart:io';

import 'package:flutter/material.dart';

/// Purely presentational Mega Drive launch sequence; ROM lookup/launch stays in Rust.
/// The perspective console and cartridge are drawn independently, so the front
/// lip can occlude the cartridge as it enters the slot.
class MegaDriveLaunchScene extends StatefulWidget {
  const MegaDriveLaunchScene({
    super.key,
    required this.title,
    required this.coverUri,
    required this.onFinished,
  });

  static const duration = Duration(milliseconds: 2200);
  final String title;
  final String? coverUri;
  final VoidCallback onFinished;

  @override
  State<MegaDriveLaunchScene> createState() => _MegaDriveLaunchSceneState();
}

class _MegaDriveLaunchSceneState extends State<MegaDriveLaunchScene>
    with SingleTickerProviderStateMixin {
  late final AnimationController _controller =
      AnimationController(
        duration: MegaDriveLaunchScene.duration,
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
  Widget build(BuildContext context) => Semantics(
    label: 'Insertion de la cartouche ${widget.title} dans la Mega Drive',
    child: ColoredBox(
      key: const Key('megadrive-launch-scene'),
      color: const Color(0xF7080B13),
      child: Center(
        child: FittedBox(
          fit: BoxFit.scaleDown,
          child: SizedBox(
            width: 360,
            height: 435,
            child: AnimatedBuilder(
              animation: _controller,
              builder: (context, _) {
                final progress = _controller.value;
                // Approach, insert, then pause briefly while the power LED lights.
                final insertion = Curves.easeInOutCubic.transform(
                  (progress / 0.82).clamp(0.0, 1.0),
                );
                return Stack(
                  clipBehavior: Clip.none,
                  children: [
                    const Positioned(
                      top: 192,
                      left: 10,
                      child: CustomPaint(
                        size: Size(340, 205),
                        painter: _ConsolePainter(),
                      ),
                    ),
                    Positioned(
                      left: 98,
                      top: 20 + 150 * insertion,
                      child: Transform.scale(
                        scale: 1 - 0.12 * insertion,
                        alignment: Alignment.bottomCenter,
                        child: _Cartridge(coverUri: widget.coverUri),
                      ),
                    ),
                    const Positioned(
                      top: 287,
                      left: 10,
                      child: CustomPaint(
                        size: Size(340, 110),
                        painter: _ConsoleLipPainter(),
                      ),
                    ),
                    Positioned(
                      left: 25,
                      right: 25,
                      bottom: 12,
                      child: Column(
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
                          const SizedBox(height: 5),
                          Text(
                            progress < 0.82
                                ? 'INSERTION DE LA CARTOUCHE'
                                : 'DÉMARRAGE',
                            style: const TextStyle(
                              color: Color(0xFF67D6DA),
                              fontSize: 10,
                              fontWeight: FontWeight.w700,
                              letterSpacing: 2.5,
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

class _Cartridge extends StatelessWidget {
  const _Cartridge({required this.coverUri});
  final String? coverUri;

  @override
  Widget build(BuildContext context) {
    final uri = coverUri;
    Widget art = const Icon(
      Icons.videogame_asset,
      color: Colors.white54,
      size: 40,
    );
    if (uri != null && uri.isNotEmpty) {
      Widget error(BuildContext context, Object error, StackTrace? stack) =>
          const Icon(Icons.image_not_supported_outlined, color: Colors.white54);
      if (uri.startsWith('https://') || uri.startsWith('http://')) {
        art = Image.network(uri, fit: BoxFit.contain, errorBuilder: error);
      } else if (uri.startsWith('file://')) {
        art = Image.file(
          File.fromUri(Uri.parse(uri)),
          fit: BoxFit.contain,
          errorBuilder: error,
        );
      }
    }
    return SizedBox(
      width: 164,
      height: 160,
      child: Stack(
        children: [
          Positioned(
            left: 6,
            right: 0,
            top: 5,
            bottom: 5,
            child: DecoratedBox(
              decoration: BoxDecoration(
                color: const Color(0xFF0A0B0D),
                border: Border.all(color: const Color(0xFF4A4D50), width: 3),
                borderRadius: const BorderRadius.vertical(
                  top: Radius.circular(12),
                  bottom: Radius.circular(4),
                ),
                boxShadow: const [
                  BoxShadow(
                    color: Colors.black87,
                    blurRadius: 18,
                    offset: Offset(10, 12),
                  ),
                ],
              ),
            ),
          ),
          Positioned(
            left: 0,
            right: 8,
            top: 0,
            bottom: 16,
            child: DecoratedBox(
              decoration: BoxDecoration(
                gradient: const LinearGradient(
                  begin: Alignment.topLeft,
                  end: Alignment.bottomRight,
                  colors: [
                    Color(0xFF4C5054),
                    Color(0xFF17191C),
                    Color(0xFF090A0C),
                  ],
                ),
                border: Border.all(color: const Color(0xFF656A6B), width: 2),
                borderRadius: const BorderRadius.vertical(
                  top: Radius.circular(14),
                  bottom: Radius.circular(3),
                ),
              ),
              child: Padding(
                padding: const EdgeInsets.fromLTRB(13, 31, 13, 12),
                child: DecoratedBox(
                  key: const Key('megadrive-cartridge-label'),
                  decoration: BoxDecoration(
                    color: const Color(0xFF181D28),
                    border: Border.all(
                      color: const Color(0xFF9AA5B0),
                      width: 2,
                    ),
                  ),
                  child: Padding(
                    padding: const EdgeInsets.all(3),
                    child: Center(child: art),
                  ),
                ),
              ),
            ),
          ),
          Positioned(
            left: 35,
            right: 40,
            bottom: 0,
            height: 14,
            child: DecoratedBox(
              decoration: BoxDecoration(
                color: const Color(0xFFA38945),
                border: Border.all(color: const Color(0xFF393019), width: 2),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _ConsolePainter extends CustomPainter {
  const _ConsolePainter();

  @override
  void paint(Canvas canvas, Size size) {
    void shape(List<Offset> points, Color color) {
      final path = Path()..addPolygon(points, true);
      canvas.drawPath(path, Paint()..color = color);
    }

    canvas.drawOval(
      const Rect.fromLTWH(18, 156, 304, 45),
      Paint()
        ..color = const Color(0xAA000000)
        ..maskFilter = const MaskFilter.blur(BlurStyle.normal, 16),
    );
    shape(const [
      Offset(16, 100),
      Offset(58, 42),
      Offset(278, 42),
      Offset(326, 100),
      Offset(295, 155),
      Offset(43, 155),
    ], const Color(0xFF191C20));
    shape(const [
      Offset(16, 100),
      Offset(43, 155),
      Offset(295, 155),
      Offset(326, 100),
      Offset(312, 132),
      Offset(286, 174),
      Offset(55, 174),
      Offset(28, 132),
    ], const Color(0xFF0C0E11));
    canvas.drawOval(
      const Rect.fromLTWH(35, 45, 270, 111),
      Paint()..color = const Color(0xFF41454A),
    );
    canvas.drawOval(
      const Rect.fromLTWH(41, 49, 258, 101),
      Paint()..color = const Color(0xFF181B1F),
    );
    canvas.drawOval(
      const Rect.fromLTWH(53, 57, 234, 85),
      Paint()..color = const Color(0xFF292C30),
    );
    canvas.drawOval(
      const Rect.fromLTWH(81, 67, 178, 60),
      Paint()..color = const Color(0xFF111316),
    );
    canvas.drawRRect(
      RRect.fromRectAndRadius(
        const Rect.fromLTWH(113, 80, 114, 16),
        const Radius.circular(6),
      ),
      Paint()..color = const Color(0xFF696A68),
    );
    canvas.drawRRect(
      RRect.fromRectAndRadius(
        const Rect.fromLTWH(119, 82, 102, 12),
        const Radius.circular(4),
      ),
      Paint()..color = Colors.black,
    );
  }

  @override
  bool shouldRepaint(covariant _ConsolePainter oldDelegate) => false;
}

class _ConsoleLipPainter extends CustomPainter {
  const _ConsoleLipPainter();

  @override
  void paint(Canvas canvas, Size size) {
    final front = Path()
      ..moveTo(28, 0)
      ..lineTo(52, 42)
      ..lineTo(287, 42)
      ..lineTo(313, 0)
      ..lineTo(298, 66)
      ..lineTo(280, 78)
      ..lineTo(59, 78)
      ..lineTo(39, 66)
      ..close();
    canvas.drawPath(front, Paint()..color = const Color(0xFF111317));
    canvas.drawLine(
      const Offset(47, 42),
      const Offset(293, 42),
      Paint()
        ..color = const Color(0xFF55575A)
        ..strokeWidth = 2,
    );
    canvas.drawRRect(
      RRect.fromRectAndRadius(
        const Rect.fromLTWH(111, 51, 118, 17),
        const Radius.circular(4),
      ),
      Paint()..color = const Color(0xFF272C32),
    );
    canvas.drawCircle(
      const Offset(278, 53),
      5,
      Paint()..color = const Color(0xFFCE3433),
    );
    canvas.drawCircle(
      const Offset(278, 53),
      9,
      Paint()
        ..color = const Color(0x66CE3433)
        ..maskFilter = const MaskFilter.blur(BlurStyle.normal, 6),
    );
  }

  @override
  bool shouldRepaint(covariant _ConsoleLipPainter oldDelegate) => false;
}
