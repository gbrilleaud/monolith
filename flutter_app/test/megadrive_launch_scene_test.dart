import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:monolith/megadrive_launch_scene.dart';

void main() {
  testWidgets('la scène Mega Drive tient sur un écran mobile', (tester) async {
    tester.view.physicalSize = const Size(390, 844);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: RepaintBoundary(
            key: const Key('preview-boundary'),
            child: MegaDriveLaunchScene(
              title: 'Jeu Mega Drive',
              coverUri: null,
              onFinished: () {},
            ),
          ),
        ),
      ),
    );
    await tester.pump(const Duration(milliseconds: 1300));
    await tester.pump(const Duration(milliseconds: 1));
    expect(tester.takeException(), isNull);
    expect(find.byKey(const Key('megadrive-launch-scene')), findsOneWidget);
    expect(find.text('Jeu Mega Drive'), findsOneWidget);
  });
}
