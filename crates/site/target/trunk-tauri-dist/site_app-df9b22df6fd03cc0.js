import { jsAppStateLoad, jsCacheGetText, jsExplorerCreateDir, jsExplorerCreateFile, jsExplorerDelete, jsExplorerListDir, jsExplorerReadTextFile, jsExplorerRequestPermission, jsExplorerStat, jsExplorerWriteTextFile, jsPrefsLoad, jsPrefsSave } from './snippets/platform_host_web-a3dd707719cd2672/inline0.js';

export class IntoUnderlyingByteSource {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        IntoUnderlyingByteSourceFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_intounderlyingbytesource_free(ptr, 0);
    }
    /**
     * @returns {number}
     */
    get autoAllocateChunkSize() {
        const ret = wasm.intounderlyingbytesource_autoAllocateChunkSize(this.__wbg_ptr);
        return ret >>> 0;
    }
    cancel() {
        const ptr = this.__destroy_into_raw();
        wasm.intounderlyingbytesource_cancel(ptr);
    }
    /**
     * @param {ReadableByteStreamController} controller
     * @returns {Promise<any>}
     */
    pull(controller) {
        const ret = wasm.intounderlyingbytesource_pull(this.__wbg_ptr, controller);
        return ret;
    }
    /**
     * @param {ReadableByteStreamController} controller
     */
    start(controller) {
        wasm.intounderlyingbytesource_start(this.__wbg_ptr, controller);
    }
    /**
     * @returns {ReadableStreamType}
     */
    get type() {
        const ret = wasm.intounderlyingbytesource_type(this.__wbg_ptr);
        return __wbindgen_enum_ReadableStreamType[ret];
    }
}
if (Symbol.dispose) IntoUnderlyingByteSource.prototype[Symbol.dispose] = IntoUnderlyingByteSource.prototype.free;

export class IntoUnderlyingSink {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        IntoUnderlyingSinkFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_intounderlyingsink_free(ptr, 0);
    }
    /**
     * @param {any} reason
     * @returns {Promise<any>}
     */
    abort(reason) {
        const ptr = this.__destroy_into_raw();
        const ret = wasm.intounderlyingsink_abort(ptr, reason);
        return ret;
    }
    /**
     * @returns {Promise<any>}
     */
    close() {
        const ptr = this.__destroy_into_raw();
        const ret = wasm.intounderlyingsink_close(ptr);
        return ret;
    }
    /**
     * @param {any} chunk
     * @returns {Promise<any>}
     */
    write(chunk) {
        const ret = wasm.intounderlyingsink_write(this.__wbg_ptr, chunk);
        return ret;
    }
}
if (Symbol.dispose) IntoUnderlyingSink.prototype[Symbol.dispose] = IntoUnderlyingSink.prototype.free;

export class IntoUnderlyingSource {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        IntoUnderlyingSourceFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_intounderlyingsource_free(ptr, 0);
    }
    cancel() {
        const ptr = this.__destroy_into_raw();
        wasm.intounderlyingsource_cancel(ptr);
    }
    /**
     * @param {ReadableStreamDefaultController} controller
     * @returns {Promise<any>}
     */
    pull(controller) {
        const ret = wasm.intounderlyingsource_pull(this.__wbg_ptr, controller);
        return ret;
    }
}
if (Symbol.dispose) IntoUnderlyingSource.prototype[Symbol.dispose] = IntoUnderlyingSource.prototype.free;
import * as import1 from "./snippets/platform_host_web-a3dd707719cd2672/inline0.js"
import * as import2 from "./snippets/platform_host_web-a3dd707719cd2672/inline0.js"
import * as import3 from "./snippets/platform_host_web-a3dd707719cd2672/inline0.js"

function __wbg_get_imports() {
    const import0 = {
        __proto__: null,
        __wbg_Error_4577686b3a6d9b3a: function(arg0, arg1) {
            var v0 = getCachedStringFromWasm0(arg0, arg1);
            const ret = Error(v0);
            return ret;
        },
        __wbg_Number_e89e48a2fe1a6355: function(arg0) {
            const ret = Number(arg0);
            return ret;
        },
        __wbg_String_8564e559799eccda: function(arg0, arg1) {
            const ret = String(arg1);
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg___wbindgen_bigint_get_as_i64_578010f8442e0319: function(arg0, arg1) {
            const v = arg1;
            const ret = typeof(v) === 'bigint' ? v : undefined;
            getDataViewMemory0().setBigInt64(arg0 + 8 * 1, isLikeNone(ret) ? BigInt(0) : ret, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, !isLikeNone(ret), true);
        },
        __wbg___wbindgen_boolean_get_18c4ed9422296fff: function(arg0) {
            const v = arg0;
            const ret = typeof(v) === 'boolean' ? v : undefined;
            return isLikeNone(ret) ? 0xFFFFFF : ret ? 1 : 0;
        },
        __wbg___wbindgen_debug_string_ddde1867f49c2442: function(arg0, arg1) {
            const ret = debugString(arg1);
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg___wbindgen_in_1064a108f4d18b9e: function(arg0, arg1) {
            const ret = arg0 in arg1;
            return ret;
        },
        __wbg___wbindgen_is_bigint_a157f0734ca85901: function(arg0) {
            const ret = typeof(arg0) === 'bigint';
            return ret;
        },
        __wbg___wbindgen_is_falsy_4eeff7cfd03a8635: function(arg0) {
            const ret = !arg0;
            return ret;
        },
        __wbg___wbindgen_is_function_d633e708baf0d146: function(arg0) {
            const ret = typeof(arg0) === 'function';
            return ret;
        },
        __wbg___wbindgen_is_null_a2a19127c13e7126: function(arg0) {
            const ret = arg0 === null;
            return ret;
        },
        __wbg___wbindgen_is_object_4b3de556756ee8a8: function(arg0) {
            const val = arg0;
            const ret = typeof(val) === 'object' && val !== null;
            return ret;
        },
        __wbg___wbindgen_is_string_7debe47dc1e045c2: function(arg0) {
            const ret = typeof(arg0) === 'string';
            return ret;
        },
        __wbg___wbindgen_is_undefined_c18285b9fc34cb7d: function(arg0) {
            const ret = arg0 === undefined;
            return ret;
        },
        __wbg___wbindgen_jsval_eq_a6afb59d8c5e78d6: function(arg0, arg1) {
            const ret = arg0 === arg1;
            return ret;
        },
        __wbg___wbindgen_jsval_loose_eq_1562ceb9af84e990: function(arg0, arg1) {
            const ret = arg0 == arg1;
            return ret;
        },
        __wbg___wbindgen_number_get_5854912275df1894: function(arg0, arg1) {
            const obj = arg1;
            const ret = typeof(obj) === 'number' ? obj : undefined;
            getDataViewMemory0().setFloat64(arg0 + 8 * 1, isLikeNone(ret) ? 0 : ret, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, !isLikeNone(ret), true);
        },
        __wbg___wbindgen_rethrow_0803fa3da1b498f1: function(arg0) {
            throw arg0;
        },
        __wbg___wbindgen_string_get_3e5751597f39a112: function(arg0, arg1) {
            const obj = arg1;
            const ret = typeof(obj) === 'string' ? obj : undefined;
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg___wbindgen_throw_39bc967c0e5a9b58: function(arg0, arg1) {
            var v0 = getCachedStringFromWasm0(arg0, arg1);
            throw new Error(v0);
        },
        __wbg__wbg_cb_unref_b6d832240a919168: function(arg0) {
            arg0._wbg_cb_unref();
        },
        __wbg_activeElement_31e766ce04adbe5e: function(arg0) {
            const ret = arg0.activeElement;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_addEventListener_ba87252e1a7558be: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            arg0.addEventListener(v0, arg3, arg4);
        }, arguments); },
        __wbg_addEventListener_c4f780106c414839: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            arg0.addEventListener(v0, arg3);
        }, arguments); },
        __wbg_add_db0285f1ee36c7f1: function() { return handleError(function (arg0, arg1, arg2) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            arg0.add(v0);
        }, arguments); },
        __wbg_altKey_6f89a54e91c24976: function(arg0) {
            const ret = arg0.altKey;
            return ret;
        },
        __wbg_altKey_dd23e9838cbfcfc8: function(arg0) {
            const ret = arg0.altKey;
            return ret;
        },
        __wbg_appendChild_f8784f6270d097cd: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.appendChild(arg1);
            return ret;
        }, arguments); },
        __wbg_append_2d44f2397a6b3cce: function() { return handleError(function (arg0, arg1) {
            arg0.append(arg1);
        }, arguments); },
        __wbg_append_b1565af0bc793fc2: function() { return handleError(function (arg0, arg1) {
            arg0.append(arg1);
        }, arguments); },
        __wbg_before_638f2de7598b61ce: function() { return handleError(function (arg0, arg1) {
            arg0.before(arg1);
        }, arguments); },
        __wbg_body_4eb4906314b12ac0: function(arg0) {
            const ret = arg0.body;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_buffer_b47db24bb1e1d5fd: function(arg0) {
            const ret = arg0.buffer;
            return ret;
        },
        __wbg_button_048e9cbb8b27af8e: function(arg0) {
            const ret = arg0.button;
            return ret;
        },
        __wbg_byobRequest_4b70301a5c6df8b7: function(arg0) {
            const ret = arg0.byobRequest;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_byteLength_262560dcfaac168c: function(arg0) {
            const ret = arg0.byteLength;
            return ret;
        },
        __wbg_byteOffset_1ae5d57ee8b19437: function(arg0) {
            const ret = arg0.byteOffset;
            return ret;
        },
        __wbg_call_08ad0d89caa7cb79: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.call(arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_call_73af281463ec8b58: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.call(arg1);
            return ret;
        }, arguments); },
        __wbg_cancelBubble_d83a60e9f0fe75c0: function(arg0) {
            const ret = arg0.cancelBubble;
            return ret;
        },
        __wbg_childNodes_10fe1c00547818b6: function(arg0) {
            const ret = arg0.childNodes;
            return ret;
        },
        __wbg_classList_4d6a49c15855e3ea: function(arg0) {
            const ret = arg0.classList;
            return ret;
        },
        __wbg_clearInterval_7d974f351c09852d: function(arg0, arg1) {
            arg0.clearInterval(arg1);
        },
        __wbg_clientX_810042c308568d39: function(arg0) {
            const ret = arg0.clientX;
            return ret;
        },
        __wbg_clientY_ddce2da9c948105f: function(arg0) {
            const ret = arg0.clientY;
            return ret;
        },
        __wbg_cloneNode_80f14b877dbe210f: function() { return handleError(function (arg0) {
            const ret = arg0.cloneNode();
            return ret;
        }, arguments); },
        __wbg_close_62e2e92a10707d34: function() { return handleError(function (arg0) {
            arg0.close();
        }, arguments); },
        __wbg_close_ef74cfcec53d2f45: function() { return handleError(function (arg0) {
            arg0.close();
        }, arguments); },
        __wbg_composedPath_80c7136c67c78587: function(arg0) {
            const ret = arg0.composedPath();
            return ret;
        },
        __wbg_createComment_92111b0673179fae: function(arg0, arg1, arg2) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            const ret = arg0.createComment(v0);
            return ret;
        },
        __wbg_createDocumentFragment_f8494d73e12e7edc: function(arg0) {
            const ret = arg0.createDocumentFragment();
            return ret;
        },
        __wbg_createElementNS_ec037330952e0beb: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            var v1 = getCachedStringFromWasm0(arg3, arg4);
            const ret = arg0.createElementNS(v0, v1);
            return ret;
        }, arguments); },
        __wbg_createElement_c28be812ac2ffe84: function() { return handleError(function (arg0, arg1, arg2) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            const ret = arg0.createElement(v0);
            return ret;
        }, arguments); },
        __wbg_createTextNode_608e1a15d944e7d8: function(arg0, arg1, arg2) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            const ret = arg0.createTextNode(v0);
            return ret;
        },
        __wbg_ctrlKey_c66665e9d705f967: function(arg0) {
            const ret = arg0.ctrlKey;
            return ret;
        },
        __wbg_ctrlKey_ff524c2e8a33ea2a: function(arg0) {
            const ret = arg0.ctrlKey;
            return ret;
        },
        __wbg_currentTarget_e1cd1e0cb13086b5: function(arg0) {
            const ret = arg0.currentTarget;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_decodeURIComponent_1fbbd3d4e73ff82e: function() { return handleError(function (arg0, arg1) {
            var v0 = getCachedStringFromWasm0(arg0, arg1);
            const ret = decodeURIComponent(v0);
            return ret;
        }, arguments); },
        __wbg_decodeURI_4dff24741ec274ee: function() { return handleError(function (arg0, arg1) {
            var v0 = getCachedStringFromWasm0(arg0, arg1);
            const ret = decodeURI(v0);
            return ret;
        }, arguments); },
        __wbg_defaultPrevented_2ec12c59ac75e753: function(arg0) {
            const ret = arg0.defaultPrevented;
            return ret;
        },
        __wbg_deleteContents_756ce75f2c4bb189: function() { return handleError(function (arg0) {
            arg0.deleteContents();
        }, arguments); },
        __wbg_document_0b7613236d782ccc: function(arg0) {
            const ret = arg0.document;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_done_5aad55ec6b1954b1: function(arg0) {
            const ret = arg0.done;
            return ret;
        },
        __wbg_enqueue_c2ddb447e86c688b: function() { return handleError(function (arg0, arg1) {
            arg0.enqueue(arg1);
        }, arguments); },
        __wbg_entries_28d32ba4cd93f5fc: function(arg0) {
            const ret = Object.entries(arg0);
            return ret;
        },
        __wbg_error_ad28debb48b5c6bb: function(arg0) {
            console.error(arg0);
        },
        __wbg_exec_ac48b0d32948e083: function(arg0, arg1, arg2) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            const ret = arg0.exec(v0);
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_focus_a5756e69ecf69851: function() { return handleError(function (arg0) {
            arg0.focus();
        }, arguments); },
        __wbg_getAttribute_0ea134ba0b840481: function(arg0, arg1, arg2, arg3) {
            var v0 = getCachedStringFromWasm0(arg2, arg3);
            const ret = arg1.getAttribute(v0);
            var ptr2 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len2 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len2, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr2, true);
        },
        __wbg_getDate_36b92ebcc42b5265: function(arg0) {
            const ret = arg0.getDate();
            return ret;
        },
        __wbg_getElementById_dff2c0f6070bc31a: function(arg0, arg1, arg2) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            const ret = arg0.getElementById(v0);
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_getFullYear_9c15c32a31fb7eb8: function(arg0) {
            const ret = arg0.getFullYear();
            return ret;
        },
        __wbg_getHours_64fe917a82d06a8e: function(arg0) {
            const ret = arg0.getHours();
            return ret;
        },
        __wbg_getItem_d794ea14168dbca6: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            var v0 = getCachedStringFromWasm0(arg2, arg3);
            const ret = arg1.getItem(v0);
            var ptr2 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len2 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len2, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr2, true);
        }, arguments); },
        __wbg_getMinutes_51eadf65f0a1a5ec: function(arg0) {
            const ret = arg0.getMinutes();
            return ret;
        },
        __wbg_getMonth_dc1d8154ce70029d: function(arg0) {
            const ret = arg0.getMonth();
            return ret;
        },
        __wbg_getSeconds_049e5b86a58c626a: function(arg0) {
            const ret = arg0.getSeconds();
            return ret;
        },
        __wbg_get_18349afdb36339a9: function() { return handleError(function (arg0, arg1) {
            const ret = Reflect.get(arg0, arg1);
            return ret;
        }, arguments); },
        __wbg_get_4920fefd3451364b: function() { return handleError(function (arg0, arg1) {
            const ret = Reflect.get(arg0, arg1);
            return ret;
        }, arguments); },
        __wbg_get_f09c3a16f8848381: function(arg0, arg1) {
            const ret = arg0[arg1 >>> 0];
            return ret;
        },
        __wbg_get_unchecked_3d0f4b91c8eca4f0: function(arg0, arg1) {
            const ret = arg0[arg1 >>> 0];
            return ret;
        },
        __wbg_get_with_ref_key_6412cf3094599694: function(arg0, arg1) {
            const ret = arg0[arg1];
            return ret;
        },
        __wbg_hasAttribute_8028d7354f1ee630: function(arg0, arg1, arg2) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            const ret = arg0.hasAttribute(v0);
            return ret;
        },
        __wbg_hash_c3b5d7caab528f98: function(arg0, arg1) {
            const ret = arg1.hash;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_hash_d749d1249013fd4d: function() { return handleError(function (arg0, arg1) {
            const ret = arg1.hash;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments); },
        __wbg_head_871365e0797471d0: function(arg0) {
            const ret = arg0.head;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_history_049e4c0db1b4c05f: function() { return handleError(function (arg0) {
            const ret = arg0.history;
            return ret;
        }, arguments); },
        __wbg_host_7e46e40ab022dab7: function(arg0) {
            const ret = arg0.host;
            return ret;
        },
        __wbg_href_72623bc1de5c4cb5: function(arg0, arg1) {
            const ret = arg1.href;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_href_abaf95e51f37650a: function(arg0, arg1) {
            const ret = arg1.href;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_id_9f9ac8e79d78f45b: function(arg0, arg1) {
            const ret = arg1.id;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_innerHeight_c56d8190ba82af0a: function() { return handleError(function (arg0) {
            const ret = arg0.innerHeight;
            return ret;
        }, arguments); },
        __wbg_innerWidth_97c0ba1978a6cef6: function() { return handleError(function (arg0) {
            const ret = arg0.innerWidth;
            return ret;
        }, arguments); },
        __wbg_instanceof_ArrayBuffer_15859862b80b732d: function(arg0) {
            let result;
            try {
                result = arg0 instanceof ArrayBuffer;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_Element_3db3f20fca28f450: function(arg0) {
            let result;
            try {
                result = arg0 instanceof Element;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_HtmlAnchorElement_1f74ed7d3eaf8b45: function(arg0) {
            let result;
            try {
                result = arg0 instanceof HTMLAnchorElement;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_HtmlElement_4e9f5820ff28f6f0: function(arg0) {
            let result;
            try {
                result = arg0 instanceof HTMLElement;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_Map_9df16bf25ceb4ecb: function(arg0) {
            let result;
            try {
                result = arg0 instanceof Map;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_ShadowRoot_845d1aa5e4cd52ce: function(arg0) {
            let result;
            try {
                result = arg0 instanceof ShadowRoot;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_Uint8Array_2240b7046ac16f05: function(arg0) {
            let result;
            try {
                result = arg0 instanceof Uint8Array;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_Window_4aba49e4d1a12365: function(arg0) {
            let result;
            try {
                result = arg0 instanceof Window;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_isArray_fad08a0d12828686: function(arg0) {
            const ret = Array.isArray(arg0);
            return ret;
        },
        __wbg_isPrimary_c780fe61bb59856a: function(arg0) {
            const ret = arg0.isPrimary;
            return ret;
        },
        __wbg_isSafeInteger_10e4151eb694e42a: function(arg0) {
            const ret = Number.isSafeInteger(arg0);
            return ret;
        },
        __wbg_is_1ad0660d6042ae31: function(arg0, arg1) {
            const ret = Object.is(arg0, arg1);
            return ret;
        },
        __wbg_item_41e4aadc3ee5e4e6: function(arg0, arg1) {
            const ret = arg0.item(arg1 >>> 0);
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_iterator_fc7ad8d33bab9e26: function() {
            const ret = Symbol.iterator;
            return ret;
        },
        __wbg_jsAppStateLoad_653090ff1e0836f4: function(arg0, arg1) {
            var v0 = getCachedStringFromWasm0(arg0, arg1);
            const ret = jsAppStateLoad(v0);
            return ret;
        },
        __wbg_jsCacheGetText_5fe3f6e34e383de9: function(arg0, arg1, arg2, arg3) {
            var v0 = getCachedStringFromWasm0(arg0, arg1);
            var v1 = getCachedStringFromWasm0(arg2, arg3);
            const ret = jsCacheGetText(v0, v1);
            return ret;
        },
        __wbg_jsExplorerCreateDir_3fb5ec5d9043e53b: function(arg0, arg1) {
            var v0 = getCachedStringFromWasm0(arg0, arg1);
            const ret = jsExplorerCreateDir(v0);
            return ret;
        },
        __wbg_jsExplorerCreateFile_81a2e3e526be9088: function(arg0, arg1, arg2, arg3) {
            var v0 = getCachedStringFromWasm0(arg0, arg1);
            var v1 = getCachedStringFromWasm0(arg2, arg3);
            const ret = jsExplorerCreateFile(v0, v1);
            return ret;
        },
        __wbg_jsExplorerDelete_96e50b78a310772b: function(arg0, arg1, arg2) {
            var v0 = getCachedStringFromWasm0(arg0, arg1);
            const ret = jsExplorerDelete(v0, arg2 !== 0);
            return ret;
        },
        __wbg_jsExplorerListDir_91502f16bd05db27: function(arg0, arg1) {
            var v0 = getCachedStringFromWasm0(arg0, arg1);
            const ret = jsExplorerListDir(v0);
            return ret;
        },
        __wbg_jsExplorerReadTextFile_5bd994f20ebf463c: function(arg0, arg1) {
            var v0 = getCachedStringFromWasm0(arg0, arg1);
            const ret = jsExplorerReadTextFile(v0);
            return ret;
        },
        __wbg_jsExplorerRequestPermission_a81eaa7cc97494c7: function(arg0, arg1) {
            var v0 = getCachedStringFromWasm0(arg0, arg1);
            const ret = jsExplorerRequestPermission(v0);
            return ret;
        },
        __wbg_jsExplorerStat_d37b4ab2ef7502e4: function(arg0, arg1) {
            var v0 = getCachedStringFromWasm0(arg0, arg1);
            const ret = jsExplorerStat(v0);
            return ret;
        },
        __wbg_jsExplorerWriteTextFile_5d642f2215b8b3be: function(arg0, arg1, arg2, arg3) {
            var v0 = getCachedStringFromWasm0(arg0, arg1);
            var v1 = getCachedStringFromWasm0(arg2, arg3);
            const ret = jsExplorerWriteTextFile(v0, v1);
            return ret;
        },
        __wbg_jsPrefsLoad_09a8dd066efc5f42: function(arg0, arg1) {
            var v0 = getCachedStringFromWasm0(arg0, arg1);
            const ret = jsPrefsLoad(v0);
            return ret;
        },
        __wbg_jsPrefsSave_8b875a90458e18fa: function(arg0, arg1, arg2, arg3) {
            var v0 = getCachedStringFromWasm0(arg0, arg1);
            var v1 = getCachedStringFromWasm0(arg2, arg3);
            const ret = jsPrefsSave(v0, v1);
            return ret;
        },
        __wbg_key_659f8d2f3a028e75: function(arg0, arg1) {
            const ret = arg1.key;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_length_400493a3ac67a358: function(arg0) {
            const ret = arg0.length;
            return ret;
        },
        __wbg_length_5855c1f289dfffc1: function(arg0) {
            const ret = arg0.length;
            return ret;
        },
        __wbg_length_a31e05262e09b7f8: function(arg0) {
            const ret = arg0.length;
            return ret;
        },
        __wbg_localStorage_71373fb4bbe7cb23: function() { return handleError(function (arg0) {
            const ret = arg0.localStorage;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments); },
        __wbg_location_e18b1d47af5ae20f: function(arg0) {
            const ret = arg0.location;
            return ret;
        },
        __wbg_log_3c5e4b64af29e724: function(arg0) {
            console.log(arg0);
        },
        __wbg_metaKey_3159d9a0a1070899: function(arg0) {
            const ret = arg0.metaKey;
            return ret;
        },
        __wbg_metaKey_665ebd3312e5ed58: function(arg0) {
            const ret = arg0.metaKey;
            return ret;
        },
        __wbg_new_09959f7b4c92c246: function(arg0) {
            const ret = new Uint8Array(arg0);
            return ret;
        },
        __wbg_new_0_a719938e6f92ddf4: function() {
            const ret = new Date();
            return ret;
        },
        __wbg_new_1213b57bccbdbb66: function(arg0, arg1) {
            var v0 = getCachedStringFromWasm0(arg0, arg1);
            const ret = new Error(v0);
            return ret;
        },
        __wbg_new_70d0dfad1388a528: function() { return handleError(function () {
            const ret = new Range();
            return ret;
        }, arguments); },
        __wbg_new_92df58a8ec3bfb6b: function() {
            const ret = new Map();
            return ret;
        },
        __wbg_new_cbee8c0d5c479eac: function() {
            const ret = new Array();
            return ret;
        },
        __wbg_new_ed69e637b553a997: function() {
            const ret = new Object();
            return ret;
        },
        __wbg_new_f36f23f20fc3c218: function(arg0, arg1, arg2, arg3) {
            var v0 = getCachedStringFromWasm0(arg0, arg1);
            var v1 = getCachedStringFromWasm0(arg2, arg3);
            const ret = new RegExp(v0, v1);
            return ret;
        },
        __wbg_new_typed_8258a0d8488ef2a2: function(arg0, arg1) {
            try {
                var state0 = {a: arg0, b: arg1};
                var cb0 = (arg0, arg1) => {
                    const a = state0.a;
                    state0.a = 0;
                    try {
                        return wasm_bindgen__convert__closures_____invoke__h15fa74814fd82a19(a, state0.b, arg0, arg1);
                    } finally {
                        state0.a = a;
                    }
                };
                const ret = new Promise(cb0);
                return ret;
            } finally {
                state0.a = state0.b = 0;
            }
        },
        __wbg_new_with_base_205924c91be0a4a2: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            var v0 = getCachedStringFromWasm0(arg0, arg1);
            var v1 = getCachedStringFromWasm0(arg2, arg3);
            const ret = new URL(v0, v1);
            return ret;
        }, arguments); },
        __wbg_new_with_byte_offset_and_length_3e6cc05444a2f3c5: function(arg0, arg1, arg2) {
            const ret = new Uint8Array(arg0, arg1 >>> 0, arg2 >>> 0);
            return ret;
        },
        __wbg_nextSibling_2f5a8c04873ca6ea: function(arg0) {
            const ret = arg0.nextSibling;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_next_a5fe6f328f7affc2: function(arg0) {
            const ret = arg0.next;
            return ret;
        },
        __wbg_next_e592122bb4ed4c67: function() { return handleError(function (arg0) {
            const ret = arg0.next();
            return ret;
        }, arguments); },
        __wbg_now_edd718b3004d8631: function() {
            const ret = Date.now();
            return ret;
        },
        __wbg_origin_41932200b9183397: function(arg0, arg1) {
            const ret = arg1.origin;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_origin_a7a87aa0de1545b0: function() { return handleError(function (arg0, arg1) {
            const ret = arg1.origin;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments); },
        __wbg_parentNode_ec260baa7551ace2: function(arg0) {
            const ret = arg0.parentNode;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_pathname_8fcdf1fa39603594: function() { return handleError(function (arg0, arg1) {
            const ret = arg1.pathname;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments); },
        __wbg_pathname_d7079a25f1d8bd7d: function(arg0, arg1) {
            const ret = arg1.pathname;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_pointerId_96b9ca3cf529c8d7: function(arg0) {
            const ret = arg0.pointerId;
            return ret;
        },
        __wbg_pointerType_a5ece5b9c957d8b9: function(arg0, arg1) {
            const ret = arg1.pointerType;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_preventDefault_d8dbb4013b32560a: function(arg0) {
            arg0.preventDefault();
        },
        __wbg_previousSibling_47d981adcbb615f8: function(arg0) {
            const ret = arg0.previousSibling;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_prototypesetcall_f034d444741426c3: function(arg0, arg1, arg2) {
            Uint8Array.prototype.set.call(getArrayU8FromWasm0(arg0, arg1), arg2);
        },
        __wbg_pushState_c970070fe9017791: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5) {
            var v0 = getCachedStringFromWasm0(arg2, arg3);
            var v1 = getCachedStringFromWasm0(arg4, arg5);
            arg0.pushState(arg1, v0, v1);
        }, arguments); },
        __wbg_querySelectorAll_510ea5fb0871baf7: function() { return handleError(function (arg0, arg1, arg2) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            const ret = arg0.querySelectorAll(v0);
            return ret;
        }, arguments); },
        __wbg_querySelector_5a9cd5c59506cf7a: function() { return handleError(function (arg0, arg1, arg2) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            const ret = arg0.querySelector(v0);
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments); },
        __wbg_queueMicrotask_2c8dfd1056f24fdc: function(arg0) {
            const ret = arg0.queueMicrotask;
            return ret;
        },
        __wbg_queueMicrotask_8985ad63815852e7: function(arg0) {
            queueMicrotask(arg0);
        },
        __wbg_removeAttribute_ec7cfe3f3feaad6c: function() { return handleError(function (arg0, arg1, arg2) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            arg0.removeAttribute(v0);
        }, arguments); },
        __wbg_removeChild_09777b63122c7c9f: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.removeChild(arg1);
            return ret;
        }, arguments); },
        __wbg_removeEventListener_357b0bf9803ecae1: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            arg0.removeEventListener(v0, arg3);
        }, arguments); },
        __wbg_remove_575c01e4788cfef7: function(arg0) {
            arg0.remove();
        },
        __wbg_remove_82deb5c82cdf36fd: function() { return handleError(function (arg0, arg1, arg2) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            arg0.remove(v0);
        }, arguments); },
        __wbg_replaceState_7c07087732286bc6: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5) {
            var v0 = getCachedStringFromWasm0(arg2, arg3);
            var v1 = getCachedStringFromWasm0(arg4, arg5);
            arg0.replaceState(arg1, v0, v1);
        }, arguments); },
        __wbg_requestAnimationFrame_a3d50e525d8e209e: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.requestAnimationFrame(arg1);
            return ret;
        }, arguments); },
        __wbg_resolve_5d61e0d10c14730a: function(arg0) {
            const ret = Promise.resolve(arg0);
            return ret;
        },
        __wbg_respond_5a41ca076b6bc10e: function() { return handleError(function (arg0, arg1) {
            arg0.respond(arg1 >>> 0);
        }, arguments); },
        __wbg_scrollIntoView_819e71f105ce614b: function(arg0) {
            arg0.scrollIntoView();
        },
        __wbg_scrollTo_1367344919f579a6: function(arg0, arg1, arg2) {
            arg0.scrollTo(arg1, arg2);
        },
        __wbg_searchParams_d631f2a80460c2d1: function(arg0) {
            const ret = arg0.searchParams;
            return ret;
        },
        __wbg_search_3b0bdaea662128ca: function() { return handleError(function (arg0, arg1) {
            const ret = arg1.search;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments); },
        __wbg_search_ad51fc9d99fe60f7: function(arg0, arg1) {
            const ret = arg1.search;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_setAttribute_52376c4b548b7c58: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            var v1 = getCachedStringFromWasm0(arg3, arg4);
            arg0.setAttribute(v0, v1);
        }, arguments); },
        __wbg_setEndBefore_20e270163e53c11f: function() { return handleError(function (arg0, arg1) {
            arg0.setEndBefore(arg1);
        }, arguments); },
        __wbg_setInterval_5f5eb1d1c6075fff: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.setInterval(arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_setItem_0c9c2d583a540407: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            var v1 = getCachedStringFromWasm0(arg3, arg4);
            arg0.setItem(v0, v1);
        }, arguments); },
        __wbg_setPointerCapture_67124f7ff96cbfcc: function() { return handleError(function (arg0, arg1) {
            arg0.setPointerCapture(arg1);
        }, arguments); },
        __wbg_setStartBefore_4a23e2fcb536b911: function() { return handleError(function (arg0, arg1) {
            arg0.setStartBefore(arg1);
        }, arguments); },
        __wbg_set_1fbeac6930844d01: function(arg0, arg1, arg2) {
            arg0.set(getArrayU8FromWasm0(arg1, arg2));
        },
        __wbg_set_4c81cfb5dc3a333c: function(arg0, arg1, arg2) {
            arg0[arg1 >>> 0] = arg2;
        },
        __wbg_set_6be42768c690e380: function(arg0, arg1, arg2) {
            arg0[arg1] = arg2;
        },
        __wbg_set_bad5c505cc70b5f8: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = Reflect.set(arg0, arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_set_cfc6de03f990decf: function(arg0, arg1, arg2) {
            const ret = arg0.set(arg1, arg2);
            return ret;
        },
        __wbg_set_data_5599cfe46e564542: function(arg0, arg1, arg2) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            arg0.data = v0;
        },
        __wbg_set_href_ebf900d9467a0d93: function() { return handleError(function (arg0, arg1, arg2) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            arg0.href = v0;
        }, arguments); },
        __wbg_set_innerHTML_a0467b14e22a2cb9: function(arg0, arg1, arg2) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            arg0.innerHTML = v0;
        },
        __wbg_set_textContent_ccd33eab05add227: function(arg0, arg1, arg2) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            arg0.textContent = v0;
        },
        __wbg_shiftKey_a05e8e0faf05efa4: function(arg0) {
            const ret = arg0.shiftKey;
            return ret;
        },
        __wbg_shiftKey_f1de6c442d6b6562: function(arg0) {
            const ret = arg0.shiftKey;
            return ret;
        },
        __wbg_state_b6c2fe8586118acb: function() { return handleError(function (arg0) {
            const ret = arg0.state;
            return ret;
        }, arguments); },
        __wbg_static_accessor_GLOBAL_THIS_14325d8cca34bb77: function() {
            const ret = typeof globalThis === 'undefined' ? null : globalThis;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_GLOBAL_f3a1e69f9c5a7e8e: function() {
            const ret = typeof global === 'undefined' ? null : global;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_SELF_50cdb5b517789aca: function() {
            const ret = typeof self === 'undefined' ? null : self;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_WINDOW_d6c4126e4c244380: function() {
            const ret = typeof window === 'undefined' ? null : window;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_stopPropagation_189d059eaef22300: function(arg0) {
            arg0.stopPropagation();
        },
        __wbg_target_56b3dcb30863f833: function(arg0, arg1) {
            const ret = arg1.target;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_target_adcfd1353c3de078: function(arg0) {
            const ret = arg0.target;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_then_d4163530723f56f4: function(arg0, arg1, arg2) {
            const ret = arg0.then(arg1, arg2);
            return ret;
        },
        __wbg_then_f1c954fe00733701: function(arg0, arg1) {
            const ret = arg0.then(arg1);
            return ret;
        },
        __wbg_value_161196e83c12d910: function(arg0, arg1) {
            const ret = arg1.value;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_value_667dcb90597486a6: function(arg0) {
            const ret = arg0.value;
            return ret;
        },
        __wbg_view_5775d5ccc7fa9230: function(arg0) {
            const ret = arg0.view;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_warn_3310c7343993c074: function(arg0) {
            console.warn(arg0);
        },
        __wbindgen_cast_0000000000000001: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { dtor_idx: 1046, function: Function { arguments: [NamedExternref("KeyboardEvent")], shim_idx: 1047, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm.wasm_bindgen__closure__destroy__h56eaebb69a747bdc, wasm_bindgen__convert__closures_____invoke__h2b7c17eea42f9400);
            return ret;
        },
        __wbindgen_cast_0000000000000002: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { dtor_idx: 1046, function: Function { arguments: [NamedExternref("MouseEvent")], shim_idx: 1047, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm.wasm_bindgen__closure__destroy__h56eaebb69a747bdc, wasm_bindgen__convert__closures_____invoke__h2b7c17eea42f9400_1);
            return ret;
        },
        __wbindgen_cast_0000000000000003: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { dtor_idx: 1165, function: Function { arguments: [NamedExternref("KeyboardEvent")], shim_idx: 1166, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm.wasm_bindgen__closure__destroy__h8d9b2c70e35723a6, wasm_bindgen__convert__closures_____invoke__h186da80596151e34);
            return ret;
        },
        __wbindgen_cast_0000000000000004: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { dtor_idx: 1165, function: Function { arguments: [NamedExternref("MouseEvent")], shim_idx: 1166, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm.wasm_bindgen__closure__destroy__h8d9b2c70e35723a6, wasm_bindgen__convert__closures_____invoke__h186da80596151e34_3);
            return ret;
        },
        __wbindgen_cast_0000000000000005: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { dtor_idx: 1369, function: Function { arguments: [], shim_idx: 1370, ret: Unit, inner_ret: Some(Unit) }, mutable: false }) -> Externref`.
            const ret = makeClosure(arg0, arg1, wasm.wasm_bindgen__closure__destroy__h8d7926f3570644dc, wasm_bindgen__convert__closures_____invoke__hc9ecfb0b51496bab);
            return ret;
        },
        __wbindgen_cast_0000000000000006: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { dtor_idx: 1427, function: Function { arguments: [NamedExternref("Event")], shim_idx: 1430, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm.wasm_bindgen__closure__destroy__h5afbbb50be4e1c5f, wasm_bindgen__convert__closures_____invoke__h0f3ad5f7f0b9f380);
            return ret;
        },
        __wbindgen_cast_0000000000000007: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { dtor_idx: 1427, function: Function { arguments: [], shim_idx: 1428, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm.wasm_bindgen__closure__destroy__h5afbbb50be4e1c5f, wasm_bindgen__convert__closures_____invoke__hd8c5bfe7eadd25b2);
            return ret;
        },
        __wbindgen_cast_0000000000000008: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { dtor_idx: 1445, function: Function { arguments: [], shim_idx: 1446, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm.wasm_bindgen__closure__destroy__h4c34a3254c080f7a, wasm_bindgen__convert__closures_____invoke__he8ee1732305fbf72);
            return ret;
        },
        __wbindgen_cast_0000000000000009: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { dtor_idx: 1472, function: Function { arguments: [Externref], shim_idx: 1473, ret: Result(Unit), inner_ret: Some(Result(Unit)) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm.wasm_bindgen__closure__destroy__h1ca2d4816d8b7bd8, wasm_bindgen__convert__closures_____invoke__h86505e4b9ba02ecd);
            return ret;
        },
        __wbindgen_cast_000000000000000a: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { dtor_idx: 515, function: Function { arguments: [NamedExternref("PointerEvent")], shim_idx: 516, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm.wasm_bindgen__closure__destroy__hbf86b8ff8a4c68ad, wasm_bindgen__convert__closures_____invoke__hc8b685056b9d6f7d);
            return ret;
        },
        __wbindgen_cast_000000000000000b: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { dtor_idx: 740, function: Function { arguments: [NamedExternref("KeyboardEvent")], shim_idx: 741, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm.wasm_bindgen__closure__destroy__h7e23f41a577a98e0, wasm_bindgen__convert__closures_____invoke__h49000c8de4998ac2);
            return ret;
        },
        __wbindgen_cast_000000000000000c: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { dtor_idx: 740, function: Function { arguments: [NamedExternref("MouseEvent")], shim_idx: 741, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm.wasm_bindgen__closure__destroy__h7e23f41a577a98e0, wasm_bindgen__convert__closures_____invoke__h49000c8de4998ac2_11);
            return ret;
        },
        __wbindgen_cast_000000000000000d: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { dtor_idx: 854, function: Function { arguments: [NamedExternref("KeyboardEvent")], shim_idx: 855, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm.wasm_bindgen__closure__destroy__h3fdefb89b74e7011, wasm_bindgen__convert__closures_____invoke__hd3bfc12981b30a14);
            return ret;
        },
        __wbindgen_cast_000000000000000e: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { dtor_idx: 854, function: Function { arguments: [NamedExternref("MouseEvent")], shim_idx: 855, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm.wasm_bindgen__closure__destroy__h3fdefb89b74e7011, wasm_bindgen__convert__closures_____invoke__hd3bfc12981b30a14_13);
            return ret;
        },
        __wbindgen_cast_000000000000000f: function(arg0) {
            // Cast intrinsic for `F64 -> Externref`.
            const ret = arg0;
            return ret;
        },
        __wbindgen_cast_0000000000000010: function(arg0) {
            // Cast intrinsic for `I64 -> Externref`.
            const ret = arg0;
            return ret;
        },
        __wbindgen_cast_0000000000000011: function(arg0, arg1) {
            var v0 = getCachedStringFromWasm0(arg0, arg1);
            // Cast intrinsic for `Ref(CachedString) -> Externref`.
            const ret = v0;
            return ret;
        },
        __wbindgen_cast_0000000000000012: function(arg0) {
            // Cast intrinsic for `U64 -> Externref`.
            const ret = BigInt.asUintN(64, arg0);
            return ret;
        },
        __wbindgen_init_externref_table: function() {
            const table = wasm.__wbindgen_externrefs;
            const offset = table.grow(4);
            table.set(0, undefined);
            table.set(offset + 0, undefined);
            table.set(offset + 1, null);
            table.set(offset + 2, true);
            table.set(offset + 3, false);
        },
    };
    return {
        __proto__: null,
        "./site_app_bg.js": import0,
        "./snippets/platform_host_web-a3dd707719cd2672/inline0.js": import1,
        "./snippets/platform_host_web-a3dd707719cd2672/inline0.js": import2,
        "./snippets/platform_host_web-a3dd707719cd2672/inline0.js": import3,
    };
}

function wasm_bindgen__convert__closures_____invoke__hc9ecfb0b51496bab(arg0, arg1) {
    wasm.wasm_bindgen__convert__closures_____invoke__hc9ecfb0b51496bab(arg0, arg1);
}

function wasm_bindgen__convert__closures_____invoke__hd8c5bfe7eadd25b2(arg0, arg1) {
    wasm.wasm_bindgen__convert__closures_____invoke__hd8c5bfe7eadd25b2(arg0, arg1);
}

function wasm_bindgen__convert__closures_____invoke__he8ee1732305fbf72(arg0, arg1) {
    wasm.wasm_bindgen__convert__closures_____invoke__he8ee1732305fbf72(arg0, arg1);
}

function wasm_bindgen__convert__closures_____invoke__h2b7c17eea42f9400(arg0, arg1, arg2) {
    wasm.wasm_bindgen__convert__closures_____invoke__h2b7c17eea42f9400(arg0, arg1, arg2);
}

function wasm_bindgen__convert__closures_____invoke__h2b7c17eea42f9400_1(arg0, arg1, arg2) {
    wasm.wasm_bindgen__convert__closures_____invoke__h2b7c17eea42f9400_1(arg0, arg1, arg2);
}

function wasm_bindgen__convert__closures_____invoke__h186da80596151e34(arg0, arg1, arg2) {
    wasm.wasm_bindgen__convert__closures_____invoke__h186da80596151e34(arg0, arg1, arg2);
}

function wasm_bindgen__convert__closures_____invoke__h186da80596151e34_3(arg0, arg1, arg2) {
    wasm.wasm_bindgen__convert__closures_____invoke__h186da80596151e34_3(arg0, arg1, arg2);
}

function wasm_bindgen__convert__closures_____invoke__h0f3ad5f7f0b9f380(arg0, arg1, arg2) {
    wasm.wasm_bindgen__convert__closures_____invoke__h0f3ad5f7f0b9f380(arg0, arg1, arg2);
}

function wasm_bindgen__convert__closures_____invoke__hc8b685056b9d6f7d(arg0, arg1, arg2) {
    wasm.wasm_bindgen__convert__closures_____invoke__hc8b685056b9d6f7d(arg0, arg1, arg2);
}

function wasm_bindgen__convert__closures_____invoke__h49000c8de4998ac2(arg0, arg1, arg2) {
    wasm.wasm_bindgen__convert__closures_____invoke__h49000c8de4998ac2(arg0, arg1, arg2);
}

function wasm_bindgen__convert__closures_____invoke__h49000c8de4998ac2_11(arg0, arg1, arg2) {
    wasm.wasm_bindgen__convert__closures_____invoke__h49000c8de4998ac2_11(arg0, arg1, arg2);
}

function wasm_bindgen__convert__closures_____invoke__hd3bfc12981b30a14(arg0, arg1, arg2) {
    wasm.wasm_bindgen__convert__closures_____invoke__hd3bfc12981b30a14(arg0, arg1, arg2);
}

function wasm_bindgen__convert__closures_____invoke__hd3bfc12981b30a14_13(arg0, arg1, arg2) {
    wasm.wasm_bindgen__convert__closures_____invoke__hd3bfc12981b30a14_13(arg0, arg1, arg2);
}

function wasm_bindgen__convert__closures_____invoke__h86505e4b9ba02ecd(arg0, arg1, arg2) {
    const ret = wasm.wasm_bindgen__convert__closures_____invoke__h86505e4b9ba02ecd(arg0, arg1, arg2);
    if (ret[1]) {
        throw takeFromExternrefTable0(ret[0]);
    }
}

function wasm_bindgen__convert__closures_____invoke__h15fa74814fd82a19(arg0, arg1, arg2, arg3) {
    wasm.wasm_bindgen__convert__closures_____invoke__h15fa74814fd82a19(arg0, arg1, arg2, arg3);
}


const __wbindgen_enum_ReadableStreamType = ["bytes"];
const IntoUnderlyingByteSourceFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_intounderlyingbytesource_free(ptr >>> 0, 1));
const IntoUnderlyingSinkFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_intounderlyingsink_free(ptr >>> 0, 1));
const IntoUnderlyingSourceFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_intounderlyingsource_free(ptr >>> 0, 1));

function addToExternrefTable0(obj) {
    const idx = wasm.__externref_table_alloc();
    wasm.__wbindgen_externrefs.set(idx, obj);
    return idx;
}

const CLOSURE_DTORS = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(state => state.dtor(state.a, state.b));

function debugString(val) {
    // primitive types
    const type = typeof val;
    if (type == 'number' || type == 'boolean' || val == null) {
        return  `${val}`;
    }
    if (type == 'string') {
        return `"${val}"`;
    }
    if (type == 'symbol') {
        const description = val.description;
        if (description == null) {
            return 'Symbol';
        } else {
            return `Symbol(${description})`;
        }
    }
    if (type == 'function') {
        const name = val.name;
        if (typeof name == 'string' && name.length > 0) {
            return `Function(${name})`;
        } else {
            return 'Function';
        }
    }
    // objects
    if (Array.isArray(val)) {
        const length = val.length;
        let debug = '[';
        if (length > 0) {
            debug += debugString(val[0]);
        }
        for(let i = 1; i < length; i++) {
            debug += ', ' + debugString(val[i]);
        }
        debug += ']';
        return debug;
    }
    // Test for built-in
    const builtInMatches = /\[object ([^\]]+)\]/.exec(toString.call(val));
    let className;
    if (builtInMatches && builtInMatches.length > 1) {
        className = builtInMatches[1];
    } else {
        // Failed to match the standard '[object ClassName]'
        return toString.call(val);
    }
    if (className == 'Object') {
        // we're a user defined class or Object
        // JSON.stringify avoids problems with cycles, and is generally much
        // easier than looping through ownProperties of `val`.
        try {
            return 'Object(' + JSON.stringify(val) + ')';
        } catch (_) {
            return 'Object';
        }
    }
    // errors
    if (val instanceof Error) {
        return `${val.name}: ${val.message}\n${val.stack}`;
    }
    // TODO we could test for more things here, like `Set`s and `Map`s.
    return className;
}

function getArrayU8FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint8ArrayMemory0().subarray(ptr / 1, ptr / 1 + len);
}

function getCachedStringFromWasm0(ptr, len) {
    if (ptr === 0) {
        return getFromExternrefTable0(len);
    } else {
        return getStringFromWasm0(ptr, len);
    }
}

let cachedDataViewMemory0 = null;
function getDataViewMemory0() {
    if (cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer.detached === true || (cachedDataViewMemory0.buffer.detached === undefined && cachedDataViewMemory0.buffer !== wasm.memory.buffer)) {
        cachedDataViewMemory0 = new DataView(wasm.memory.buffer);
    }
    return cachedDataViewMemory0;
}

function getFromExternrefTable0(idx) { return wasm.__wbindgen_externrefs.get(idx); }

function getStringFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return decodeText(ptr, len);
}

let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function handleError(f, args) {
    try {
        return f.apply(this, args);
    } catch (e) {
        const idx = addToExternrefTable0(e);
        wasm.__wbindgen_exn_store(idx);
    }
}

function isLikeNone(x) {
    return x === undefined || x === null;
}

function makeClosure(arg0, arg1, dtor, f) {
    const state = { a: arg0, b: arg1, cnt: 1, dtor };
    const real = (...args) => {

        // First up with a closure we increment the internal reference
        // count. This ensures that the Rust closure environment won't
        // be deallocated while we're invoking it.
        state.cnt++;
        try {
            return f(state.a, state.b, ...args);
        } finally {
            real._wbg_cb_unref();
        }
    };
    real._wbg_cb_unref = () => {
        if (--state.cnt === 0) {
            state.dtor(state.a, state.b);
            state.a = 0;
            CLOSURE_DTORS.unregister(state);
        }
    };
    CLOSURE_DTORS.register(real, state, state);
    return real;
}

function makeMutClosure(arg0, arg1, dtor, f) {
    const state = { a: arg0, b: arg1, cnt: 1, dtor };
    const real = (...args) => {

        // First up with a closure we increment the internal reference
        // count. This ensures that the Rust closure environment won't
        // be deallocated while we're invoking it.
        state.cnt++;
        const a = state.a;
        state.a = 0;
        try {
            return f(a, state.b, ...args);
        } finally {
            state.a = a;
            real._wbg_cb_unref();
        }
    };
    real._wbg_cb_unref = () => {
        if (--state.cnt === 0) {
            state.dtor(state.a, state.b);
            state.a = 0;
            CLOSURE_DTORS.unregister(state);
        }
    };
    CLOSURE_DTORS.register(real, state, state);
    return real;
}

function passStringToWasm0(arg, malloc, realloc) {
    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8ArrayMemory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }
    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = cachedTextEncoder.encodeInto(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

function takeFromExternrefTable0(idx) {
    const value = wasm.__wbindgen_externrefs.get(idx);
    wasm.__externref_table_dealloc(idx);
    return value;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
cachedTextDecoder.decode();
const MAX_SAFARI_DECODE_BYTES = 2146435072;
let numBytesDecoded = 0;
function decodeText(ptr, len) {
    numBytesDecoded += len;
    if (numBytesDecoded >= MAX_SAFARI_DECODE_BYTES) {
        cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
        cachedTextDecoder.decode();
        numBytesDecoded = len;
    }
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

const cachedTextEncoder = new TextEncoder();

if (!('encodeInto' in cachedTextEncoder)) {
    cachedTextEncoder.encodeInto = function (arg, view) {
        const buf = cachedTextEncoder.encode(arg);
        view.set(buf);
        return {
            read: arg.length,
            written: buf.length
        };
    };
}

let WASM_VECTOR_LEN = 0;

let wasmModule, wasm;
function __wbg_finalize_init(instance, module) {
    wasm = instance.exports;
    wasmModule = module;
    cachedDataViewMemory0 = null;
    cachedUint8ArrayMemory0 = null;
    wasm.__wbindgen_start();
    return wasm;
}

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);
            } catch (e) {
                const validResponse = module.ok && expectedResponseType(module.type);

                if (validResponse && module.headers.get('Content-Type') !== 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else { throw e; }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);
    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };
        } else {
            return instance;
        }
    }

    function expectedResponseType(type) {
        switch (type) {
            case 'basic': case 'cors': case 'default': return true;
        }
        return false;
    }
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (module !== undefined) {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports();
    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }
    const instance = new WebAssembly.Instance(module, imports);
    return __wbg_finalize_init(instance, module);
}

async function __wbg_init(module_or_path) {
    if (wasm !== undefined) return wasm;


    if (module_or_path !== undefined) {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (module_or_path === undefined) {
        module_or_path = new URL('site_app_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync, __wbg_init as default };
