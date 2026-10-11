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
