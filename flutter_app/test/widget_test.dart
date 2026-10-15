import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:monolith/library_view.dart';
import 'package:monolith/style_theme.dart';
import 'package:monolith/src/rust/flutter_api.dart';
import 'package:monolith/src/rust/library_service.dart';

void main() {
  testWidgets(
    'boutons émulateurs, manettes et apparence accessibles sur écran étroit',
    (tester) async {
      tester.view.physicalSize = const Size(320, 844);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      var emulatorClicks = 0;
      var controllerClicks = 0;
      var styleClicks = 0;
      await tester.pumpWidget(
        MaterialApp(
          home: MonolithLibraryView(
            sections: const [],
            onLaunch: (_) {},
            onEmulators: () => emulatorClicks++,
            onControllers: () => controllerClicks++,
            onStyle: () => styleClicks++,
          ),
        ),
      );
      expect(tester.takeException(), isNull);
      await tester.tap(find.text('Émulateurs'));
      await tester.tap(find.text('Manettes'));
      await tester.tap(find.text('Apparence'));
      expect(emulatorClicks, 1);
      expect(controllerClicks, 1);
      expect(styleClicks, 1);
    },
  );
  testWidgets(
    'mise à jour Flutter séparée, explicite et accessible sur mobile',
    (tester) async {
      tester.view.physicalSize = const Size(390, 844);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      var clicks = 0;
      Future<void> show(String phase) async {
        await tester.pumpWidget(
          MaterialApp(
            home: MonolithLibraryView(
              sections: const [],
              onLaunch: (_) {},
              update: FlutterUpdateStatus(phase: phase, version: '0.2.0'),
              onUpdate: () => clicks++,
            ),
          ),
        );
      }

      await show('available');
      expect(find.text('Télécharger Monolith 0.2.0'), findsOneWidget);
      await tester.tap(find.text('Télécharger Monolith 0.2.0'));
      expect(clicks, 1);
      await show('ready');
      expect(
        find.text('Installer la dernière version de Monolith'),
        findsOneWidget,
      );
      await tester.tap(find.text('Installer la dernière version de Monolith'));
      expect(clicks, 2);
      await show('downloading');
      expect(
        tester
            .widget<OutlinedButton>(
              find.ancestor(
                of: find.text('Téléchargement de Monolith…'),
                matching: find.byType(OutlinedButton),
              ),
            )
            .onPressed,
        isNull,
      );
      await show('unsupported');
      expect(find.textContaining('mise à jour de Monolith'), findsNothing);
      expect(tester.takeException(), isNull);
    },
  );

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

  testWidgets(
    'grille adaptative puis liste conservent les jeux et les actions',
    (tester) async {
      tester.view.physicalSize = const Size(900, 780);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final downloads = <int>[];
      final games = List.generate(
        17,
        (i) => LibraryGame(
          gameId: i + 1,
          systemId: 11,
          title: 'Jeu $i',
          path: '',
          available: false,
          description: '',
        ),
      );
      await tester.pumpWidget(
        MaterialApp(
          home: MonolithLibraryView(
            sections: [
              LibrarySection(systemId: 11, name: 'Sega Genesis', games: games),
            ],
            onLaunch: (_) {},
            onDownload: downloads.add,
          ),
        ),
      );
      expect(tester.takeException(), isNull);
      expect(find.byType(GridView), findsOneWidget);
      expect(find.text('Télécharger'), findsWidgets);
      await tester.tap(find.byTooltip('Vue liste'));
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      expect(find.byType(GridView), findsNothing);
      expect(find.text('Jeu 0'), findsOneWidget);
      await tester.tap(find.text('Télécharger').first);
      expect(downloads, [1]);
    },
  );

  testWidgets('jaquette conservée entière sans arrondi', (tester) async {
    tester.view.physicalSize = const Size(600, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    await tester.pumpWidget(
      MaterialApp(
        home: MonolithLibraryView(
          sections: const [
            LibrarySection(
              systemId: 11,
              name: 'Genesis',
              games: [
                LibraryGame(
                  gameId: 1,
                  systemId: 11,
                  title: 'Sonic',
                  path: '',
                  coverUri: 'https://example.invalid/cover.png',
                  available: false,
                  description: '',
                ),
              ],
            ),
          ],
          onLaunch: (_) {},
        ),
      ),
    );
    final image = tester.widget<Image>(find.byType(Image).first);
    expect(image.fit, BoxFit.contain);
    final card = tester.widget<Container>(
      find
          .ancestor(of: find.text('Sonic'), matching: find.byType(Container))
          .first,
    );
    expect((card.decoration! as BoxDecoration).borderRadius, BorderRadius.zero);
  });

  testWidgets('bibliothèque applique palette et écartement des tuiles', (
    tester,
  ) async {
    final theme = ThemeData.light().copyWith(
      scaffoldBackgroundColor: const Color(0xFFF8EAD0),
      colorScheme: ColorScheme.fromSeed(seedColor: const Color(0xFF13579B)),
      extensions: const [
        StyleTokens(
          tileSpacing: 19,
          cornerRadius: 4,
          surfaceRaised: Color(0xFFE3D2C1),
          textMuted: Color(0xFF777777),
        ),
      ],
    );
    await tester.pumpWidget(
      MaterialApp(
        theme: theme,
        home: MonolithLibraryView(
          sections: const [
            LibrarySection(
              systemId: 11,
              name: 'Mega Drive',
              games: [
                LibraryGame(
                  systemId: 11,
                  title: 'Sonic',
                  path: '/roms/sonic.md',
                  available: true,
                  description: '',
                ),
              ],
            ),
          ],
          onLaunch: (_) {},
        ),
      ),
    );
    expect(
      tester.widget<Scaffold>(find.byType(Scaffold)).backgroundColor,
      const Color(0xFFF8EAD0),
    );
    final grid = tester.widget<GridView>(find.byType(GridView));
    expect(
      (grid.gridDelegate as SliverGridDelegateWithFixedCrossAxisCount)
          .crossAxisSpacing,
      19,
    );
  });

  testWidgets(
    'Mega Drive : cartouche avec jaquette, lancement différé unique',
    (tester) async {
      tester.view.physicalSize = const Size(390, 844);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final launches = <String>[];
      await tester.pumpWidget(
        MaterialApp(
          home: MonolithLibraryView(
            sections: const [
              LibrarySection(
                systemId: 11,
                name: 'Mega Drive',
                games: [
                  LibraryGame(
                    systemId: 11,
                    title: 'Sonic',
                    path: '/roms/sonic.md',
                    coverUri: 'https://example.invalid/sonic.png',
                    available: true,
                    description: '',
                  ),
                ],
              ),
            ],
            onLaunch: launches.add,
          ),
        ),
      );
      await tester.tap(find.text('Lancer'));
      await tester.pump();
      expect(find.byKey(const Key('megadrive-launch-scene')), findsOneWidget);
      final cartridgeArt = tester.widget<Image>(
        find.descendant(
          of: find.byKey(const Key('megadrive-cartridge-label')),
          matching: find.byType(Image),
        ),
      );
      expect(
        (cartridgeArt.image as NetworkImage).url,
        'https://example.invalid/sonic.png',
      );
      expect(launches, isEmpty);
      await tester.pump(const Duration(milliseconds: 1100));
      expect(launches, isEmpty);
      await tester.pump(const Duration(milliseconds: 1100));
      await tester.pump(const Duration(milliseconds: 100));
      expect(launches, ['/roms/sonic.md']);
      expect(find.byKey(const Key('megadrive-launch-scene')), findsNothing);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('Mega Drive : animation désactivée lance immédiatement', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(390, 844);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final launches = <String>[];
    await tester.pumpWidget(
      MaterialApp(
        home: MonolithLibraryView(
          sections: const [
            LibrarySection(
              systemId: 11,
              name: 'Mega Drive',
              games: [
                LibraryGame(
                  systemId: 11,
                  title: 'Sonic',
                  path: '/roms/sonic.md',
                  available: true,
                  description: '',
                ),
              ],
            ),
          ],
          onLaunch: launches.add,
          cartridgeAnimation: false,
        ),
      ),
    );
    await tester.tap(find.text('Lancer'));
    await tester.pump();
    expect(launches, ['/roms/sonic.md']);
    expect(find.byKey(const Key('megadrive-launch-scene')), findsNothing);
  });

  testWidgets('Mega Drive : lancement annulé si la vue disparaît', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(390, 844);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final launches = <String>[];
    await tester.pumpWidget(
      MaterialApp(
        home: MonolithLibraryView(
          sections: const [
            LibrarySection(
              systemId: 11,
              name: 'Sega Genesis',
              games: [
                LibraryGame(
                  systemId: 11,
                  title: 'Sonic',
                  path: '/roms/sonic.md',
                  available: true,
                  description: '',
                ),
              ],
            ),
          ],
          onLaunch: launches.add,
        ),
      ),
    );
    await tester.tap(find.text('Lancer'));
    await tester.pump();
    expect(find.byKey(const Key('megadrive-launch-scene')), findsOneWidget);
    await tester.pumpWidget(const MaterialApp(home: SizedBox()));
    await tester.pump(const Duration(seconds: 3));
    expect(launches, isEmpty);
  });

  testWidgets('la session active est identifiée et peut être fermée', (
    tester,
  ) async {
    var loggedOut = false;
    await tester.pumpWidget(
      MaterialApp(
        home: MonolithLibraryView(
          sections: const [],
          onLaunch: (_) {},
          identity: const FlutterIdentity(
            userId: 42,
            username: 'Guillaume',
            role: 'standard',
          ),
          onLogout: () => loggedOut = true,
        ),
      ),
    );
    expect(find.text('Guillaume'), findsOneWidget);
    await tester.tap(find.text('Déconnexion'));
    expect(loggedOut, isTrue);
  });
}
