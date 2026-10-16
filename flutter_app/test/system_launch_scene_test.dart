import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:monolith/system_launch_scene.dart';

void main() {
  test('tous les systèmes du NAS et les cibles PC/PS2 ont une identité de lancement', () {
    const systems = <String, String>{
      'atari_lynx': 'Lynx',
      'final_burn_alpha': 'FinalBurn Alpha',
      'mame': 'MAME',
      'nintendo_64': 'Nintendo 64',
      'nintendo_entertainment_system': 'NES',
      'nintendo_game_boy': 'Game Boy',
      'nintendo_game_boy_advance': 'Game Boy Advance',
      'nintendo_game_boy_color': 'Game Boy Color',
      'nintendo_gamecube': 'GameCube',
      'sega_dreamcast': 'Dreamcast',
      'sega_game_gear': 'Game Gear',
      'sega_master_system': 'Master System',
      'sega_saturn': 'Saturn',
      'sony_playstation': 'PlayStation',
      'sony_psp': 'PSP',
      'super_nintendo_entertainment_system': 'Super Nintendo',
      'PlayStation 2': 'PlayStation 2',
      'PC': 'PC',
    };
    for (final entry in systems.entries) {
      final identity = LaunchIdentity.forSystem(entry.key);
      expect(identity.label, entry.value, reason: entry.key);
      expect(identity.accent, isNot(Colors.transparent));
    }
    expect(LaunchIdentity.forSystem('Mystery Box').label, 'Mystery Box');
    expect(
      LaunchIdentity.forSystem('Sony PlayStation 2').label,
      'PlayStation 2',
    );
    expect(LaunchIdentity.forSystem('Sega Genesis').isMegaDrive, isTrue);
    expect(
      LaunchIdentity.forSystem('Nintendo Game Boy Color').label,
      'Game Boy Color',
    );
  });

  testWidgets('scène réactive sur petit écran, lancement différé et unique', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(320, 568);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    var finished = 0;
    await tester.pumpWidget(
      MaterialApp(
        home: SystemLaunchScene(
          systemName: 'Dreamcast',
          title: 'Rayman 2',
          coverUri: null,
          onFinished: () => finished++,
        ),
      ),
    );
    expect(find.byKey(const Key('system-launch-scene')), findsOneWidget);
    expect(find.text('DREAMCAST'), findsOneWidget);
    expect(finished, 0);
    await tester.pump(const Duration(milliseconds: 1));
    await tester.pump(SystemLaunchScene.duration);
    await tester.pump();
    expect(finished, 1);
    expect(tester.takeException(), isNull);
  });
}
