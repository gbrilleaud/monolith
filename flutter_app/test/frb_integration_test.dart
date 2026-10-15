import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart'
    show ExternalLibrary;
import 'package:monolith/src/rust/flutter_api.dart' as api;
import 'package:monolith/src/rust/frb_generated.dart';

void main() {
  final library = Platform.environment['MONOLITH_FRB_LIB'];
  test(
    'FRB charge le vrai moteur Rust et ouvre un catalogue hors ligne',
    () async {
      final directory = await Directory.systemTemp.createTemp('monolith-frb-');
      try {
        await RustLib.init(externalLibrary: ExternalLibrary.open(library!));
        final sections = await api.loadLibrary(
          dataDirectory: directory.path,
          query: 'Ray*',
        );
        expect(sections, isEmpty);
        expect(await File('${directory.path}/monolith.db').exists(), isTrue);
        final style = jsonDecode(
          await api.styleSnapshotJson(dataDirectory: directory.path),
        ) as Map<String, dynamic>;
        expect((style['catalog'] as Map)['presets'], hasLength(7));
        expect(style['selected'], 'dark');
        final selected = jsonDecode(
          await api.selectStyle(dataDirectory: directory.path, id: 'light'),
        ) as Map<String, dynamic>;
        expect(selected['resolved']['id'], 'light');
        final animation = jsonDecode(
          await api.setCartridgeAnimation(
            dataDirectory: directory.path,
            enabled: false,
          ),
        ) as Map<String, dynamic>;
        expect(animation['cartridge_animation'], isFalse);
        await File('${directory.path}/style.json').writeAsString('{broken');
        await expectLater(
          api.styleSnapshotJson(dataDirectory: directory.path),
          throwsA(isA<Exception>()),
        );
        final recovered = jsonDecode(
          await api.resetStyle(dataDirectory: directory.path),
        ) as Map<String, dynamic>;
        expect(recovered['selected'], 'dark');
        expect(recovered['cartridge_animation'], isTrue);
        expect(
          await api.sessionIdentity(dataDirectory: directory.path),
          isNull,
        );
        final originalUrl = await api.backendUrl(dataDirectory: directory.path);
        expect(originalUrl, isNotEmpty);
        await api.saveBackendUrl(
          dataDirectory: directory.path,
          value: 'http://127.0.0.1:8787/',
        );
        expect(
          await api.backendUrl(dataDirectory: directory.path),
          'http://127.0.0.1:8787',
        );
        await api.logout(dataDirectory: directory.path);
        expect(await File('${directory.path}/session.json').exists(), isFalse);
        if (!Platform.isWindows) {
          final update = await api.flutterUpdateStatus(
            dataDirectory: directory.path,
          );
          expect(update.phase, 'unsupported');
          await expectLater(
            api.installFlutterUpdate(dataDirectory: directory.path),
            throwsA(isA<Exception>()),
          );
        }
      } finally {
        RustLib.dispose();
        await directory.delete(recursive: true);
      }
    },
    skip: library == null
        ? 'Nécessite MONOLITH_FRB_LIB construit par Cargo'
        : false,
  );
}
