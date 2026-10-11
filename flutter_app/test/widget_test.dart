import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:monolith/library_view.dart';
import 'package:monolith/src/rust/library_service.dart';

void main() {
  testWidgets('bibliothèque mobile affiche les jeux et lance via callback', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(390, 844);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final selections = <String>[];
    await tester.pumpWidget(
      MaterialApp(
        home: MonolithLibraryView(
          sections: const [
            LibrarySection(
              systemId: 7,
              name: 'Dreamcast',
              games: [
                LibraryGame(
                  gameId: 101,
                  systemId: 7,
                  title: 'Rayman 2',
                  path: '/roms/rayman.gdi',
                  available: true,
                  description: 'Aventure française',
                ),
              ],
            ),
          ],
          onLaunch: selections.add,
        ),
      ),
    );
    expect(find.text('Dreamcast'), findsWidgets);
    expect(find.text('Rayman 2'), findsOneWidget);
    expect(tester.takeException(), isNull);
    await tester.tap(find.text('Lancer'));
    expect(selections, ['/roms/rayman.gdi']);
  });
}
