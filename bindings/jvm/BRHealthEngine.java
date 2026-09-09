// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

package br.health;

import java.lang.foreign.*;
import java.lang.invoke.MethodHandle;

/**
 * Interface Java 21+ de alta performance com o motor BRHealth via Project Panama (FFM API).
 *
 * Elimina o overhead do JNI tradicional permitindo intercâmbio de memória nativa contígua.
 */
public final class BRHealthEngine implements AutoCloseable {

    private final Arena arena;
    private final MethodHandle calculateDvHandle;
    private final MethodHandle classifyCsapHandle;
    private final MethodHandle latLngToH3Handle;
    private final MethodHandle computeRoiHandle;

    public BRHealthEngine(String libraryPath) {
        this.arena = Arena.ofShared();
        SymbolLookup lookup = SymbolLookup.libraryLookup(libraryPath, arena);
        Linker linker = Linker.nativeLinker();

        MemorySegment calcDvAddr = lookup.find("brhealth_panama_calculate_dv")
                .orElseThrow(() -> new UnsatisfiedLinkError("brhealth_panama_calculate_dv not found"));
        this.calculateDvHandle = linker.downcallHandle(
                calcDvAddr,
                FunctionDescriptor.of(ValueLayout.JAVA_INT, ValueLayout.ADDRESS)
        );

        MemorySegment classifyCsapAddr = lookup.find("brhealth_panama_classify_csap")
                .orElseThrow(() -> new UnsatisfiedLinkError("brhealth_panama_classify_csap not found"));
        this.classifyCsapHandle = linker.downcallHandle(
                classifyCsapAddr,
                FunctionDescriptor.of(ValueLayout.JAVA_INT, ValueLayout.ADDRESS)
        );

        MemorySegment h3Addr = lookup.find("brhealth_panama_latlng_to_h3")
                .orElseThrow(() -> new UnsatisfiedLinkError("brhealth_panama_latlng_to_h3 not found"));
        this.latLngToH3Handle = linker.downcallHandle(
                h3Addr,
                FunctionDescriptor.of(ValueLayout.JAVA_LONG, ValueLayout.JAVA_DOUBLE, ValueLayout.JAVA_DOUBLE, ValueLayout.JAVA_BYTE)
        );

        MemorySegment roiAddr = lookup.find("brhealth_panama_compute_roi")
                .orElseThrow(() -> new UnsatisfiedLinkError("brhealth_panama_compute_roi not found"));
        this.computeRoiHandle = linker.downcallHandle(
                roiAddr,
                FunctionDescriptor.of(ValueLayout.JAVA_DOUBLE, ValueLayout.JAVA_DOUBLE, ValueLayout.JAVA_DOUBLE, ValueLayout.JAVA_DOUBLE)
        );
    }

    public int calculateIbgeDv(String code6Digits) throws Throwable {
        MemorySegment cStr = arena.allocateFrom(code6Digits);
        return (int) calculateDvHandle.invokeExact(cStr);
    }

    public int classifyCsap(String cid10) throws Throwable {
        MemorySegment cStr = arena.allocateFrom(cid10);
        return (int) classifyCsapHandle.invokeExact(cStr);
    }

    public long latLngToH3(double lat, double lng, byte resolution) throws Throwable {
        return (long) latLngToH3Handle.invokeExact(lat, lng, resolution);
    }

    public double computePrimaryCareRoi(double avoidableCost, double investment, double attributableFraction) throws Throwable {
        return (double) computeRoiHandle.invokeExact(avoidableCost, investment, attributableFraction);
    }

    @Override
    public void close() {
        arena.close();
    }
}
