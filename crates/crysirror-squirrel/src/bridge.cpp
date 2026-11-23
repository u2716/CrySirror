// SPDX-License-Identifier: MIT
// No file/system libraries or bytecode loader are exposed. Squirrel owns only
// logical values; the host retains all device and storage authority.
#include <cstdlib>
#include "sqpcheader.h"
#include "sqvm.h"
#include "sqstate.h"
#include <new>
#include <cstdint>
#include <cstring>
#include <cstdio>
#include <vector>
#include <algorithm>

struct Arena;
struct alignas(std::max_align_t) Block { Arena* owner; Block* prev; Block* next; size_t size; };
struct Arena { Block* head=nullptr; size_t bytes=0; size_t limit=32*1024*1024; };
static thread_local Arena* current_arena=nullptr;
struct Guard { Arena* old; Guard(Arena* a):old(current_arena){current_arena=a;} ~Guard(){current_arena=old;} };
static void unlink_block(Block* b) { auto a=b->owner; if(b->prev)b->prev->next=b->next;else a->head=b->next;if(b->next)b->next->prev=b->prev;a->bytes-=b->size+sizeof(Block); }
void* sq_vm_malloc(SQUnsignedInteger n) {
    auto a=current_arena;
    if(!a || n>a->limit || sizeof(Block)>a->limit-n || a->bytes>a->limit-n-sizeof(Block)) throw std::bad_alloc();
    auto b=static_cast<Block*>(std::malloc(sizeof(Block)+n));if(!b)throw std::bad_alloc();
    b->owner=a;b->prev=nullptr;b->next=a->head;b->size=n;if(a->head)a->head->prev=b;a->head=b;a->bytes+=n+sizeof(Block);return b+1;
}
void sq_vm_free(void* p,SQUnsignedInteger) {if(!p)return;auto b=static_cast<Block*>(p)-1;unlink_block(b);std::free(b);}
void* sq_vm_realloc(void* p,SQUnsignedInteger,SQUnsignedInteger n) {
    if(!p)return sq_vm_malloc(n);if(!n){sq_vm_free(p,0);return nullptr;}
    auto b=static_cast<Block*>(p)-1;Guard g(b->owner);auto q=sq_vm_malloc(n);std::memcpy(q,p,std::min<size_t>(n,b->size));sq_vm_free(p,0);return q;
}
using Callback=int(*)(void*,void*);
struct Instance {
    Arena arena; HSQUIRRELVM vm=nullptr; Callback callback; void* context;
    std::vector<HSQOBJECT> refs; bool poisoned=false; char error[4096]{};
    Instance(Callback cb,void* ctx):callback(cb),context(ctx){}
    void fail(const char* message){std::snprintf(error,sizeof(error),"%s",message?message:"Squirrel error");}
    void last(){sq_getlasterror(vm);const char* message=nullptr;if(SQ_SUCCEEDED(sq_getstring(vm,-1,&message)))fail(message);else fail("Squirrel execution failed");sq_pop(vm,1);}
};
static SQInteger host_call(HSQUIRRELVM vm) {
    auto s=static_cast<Instance*>(sq_getforeignptr(vm));
    if(!s || s->poisoned)return sq_throwerror(vm,"script host unavailable");
    auto result=s->callback(s->context,s);
    if(result<0 || s->poisoned)return sq_throwerror(vm,s->error[0]?s->error:"script host rejected call");
    return result;
}
static void compiler_error(HSQUIRRELVM vm,const char* message,const char*,SQInteger line,SQInteger col) {
    auto s=static_cast<Instance*>(sq_getforeignptr(vm));std::snprintf(s->error,sizeof(s->error),"Squirrel compile %lld:%lld: %s",(long long)line,(long long)col,message);
}
#define ENTRY auto s=static_cast<Instance*>(p);if(!s || s->poisoned)return -1;Guard guard(&s->arena);try
#define FAILED catch(...){s->poisoned=true;s->fail("Squirrel heap budget/allocation failure");return -1;}
extern "C" {
void* csq_new(Callback callback,void* context) {
    auto s=new(std::nothrow)Instance(callback,context);if(!s)return nullptr;
    Guard guard(&s->arena);
    try {
        s->vm=sq_open(256);if(!s->vm)throw std::bad_alloc();sq_setforeignptr(s->vm,s);sq_setcompilererrorhandler(s->vm,compiler_error);
        sq_pushroottable(s->vm);sq_pushstring(s->vm,"__host",-1);sq_newclosure(s->vm,host_call,0);sq_newslot(s->vm,-3,SQFalse);sq_pop(s->vm,1);
        // Keep scripts single-owner and prevent environment/error-hook mutation.
        for(auto name:{"newthread","seterrorhandler","setdebughook","compilestring","getstackinfos"}) {
            sq_pushroottable(s->vm);sq_pushstring(s->vm,name,-1);sq_deleteslot(s->vm,-2,SQFalse);sq_pop(s->vm,1);
        }
        return s;
    } catch(...) { while(s->arena.head){auto b=s->arena.head;unlink_block(b);std::free(b);}delete s;return nullptr; }
}
void csq_free(void* p) {
    auto s=static_cast<Instance*>(p);if(!s)return;Guard guard(&s->arena);
    if(!s->poisoned){try{for(auto &r:s->refs)sq_release(s->vm,&r);sq_close(s->vm);}catch(...){}}
    while(s->arena.head){auto b=s->arena.head;unlink_block(b);std::free(b);}delete s;
}
const char* csq_error(void* p){return static_cast<Instance*>(p)->error;}
void csq_fail(void* p,const char* text){static_cast<Instance*>(p)->fail(text);}
int csq_eval(void* p,const char* text,size_t size,uint64_t budget) {
    ENTRY {auto v=s->vm;auto top=sq_gettop(v);_ss(v)->crysirror_budget=budget;s->error[0]=0;
        if(SQ_FAILED(sq_compilebuffer(v,text,size,"native-ui",SQTrue))){sq_settop(v,top);return -1;}
        sq_pushroottable(v);if(SQ_FAILED(sq_call(v,1,SQFalse,SQFalse))){s->last();sq_settop(v,top);return -1;}sq_settop(v,top);return 0;
    } FAILED
}
int csq_call(void* p,const char* name,int reference,uint64_t budget) {
    ENTRY {auto v=s->vm;auto top=sq_gettop(v);_ss(v)->crysirror_budget=budget;s->error[0]=0;
        if(reference>=0){if((size_t)reference>=s->refs.size()){s->fail("Squirrel callback reference range");return -1;}sq_pushobject(v,s->refs[reference]);}
        else{sq_pushroottable(v);sq_pushstring(v,name,-1);if(SQ_FAILED(sq_get(v,-2))){sq_settop(v,top);s->fail("Squirrel callback is absent");return -1;}sq_remove(v,-2);}
        sq_pushroottable(v);if(SQ_FAILED(sq_call(v,1,SQFalse,SQFalse))){s->last();sq_settop(v,top);return -1;}sq_settop(v,top);return 0;
    } FAILED
}
int csq_top(void* p){return (int)sq_gettop(static_cast<Instance*>(p)->vm);}
int csq_kind(void* p,int index){auto v=static_cast<Instance*>(p)->vm;switch(sq_gettype(v,index)){
    case OT_NULL:return 0;case OT_BOOL:return 1;case OT_INTEGER:return 2;case OT_FLOAT:return 3;case OT_STRING:return 4;case OT_TABLE:return 5;case OT_ARRAY:return 6;case OT_CLOSURE:return 7;case OT_NATIVECLOSURE:return 7;default:return -1;}}
int64_t csq_integer(void* p,int index){SQInteger n=0;sq_getinteger(static_cast<Instance*>(p)->vm,index,&n);return n;}
int csq_push_integer(void* p,int64_t n){ENTRY {sq_pushinteger(s->vm,(SQInteger)n);return 0;} FAILED}
double csq_number(void* p,int index){auto v=static_cast<Instance*>(p)->vm;SQFloat n=0;sq_getfloat(v,index,&n);return n;}
int csq_bool(void* p,int index){SQBool n=0;sq_getbool(static_cast<Instance*>(p)->vm,index,&n);return n!=0;}
const char* csq_string(void* p,int index,size_t* size){auto v=static_cast<Instance*>(p)->vm;const char* out=nullptr;SQInteger n=0;if(SQ_FAILED(sq_getstringandsize(v,index,&out,&n)))return nullptr;*size=n;return out;}
int csq_next(void* p,int index){ENTRY {return SQ_SUCCEEDED(sq_next(s->vm,index))?1:0;} FAILED}
void csq_pop(void* p,int count){sq_pop(static_cast<Instance*>(p)->vm,count);}
int csq_ref(void* p,int index){ENTRY {HSQOBJECT obj;if(SQ_FAILED(sq_getstackobj(s->vm,index,&obj)))return -1;
    for(size_t i=0;i<s->refs.size();++i)if(s->refs[i]._unVal.pRefCounted==obj._unVal.pRefCounted)return (int)i;
    if(s->refs.size()>=4096){s->fail("Squirrel callback count budget");return -1;}s->refs.push_back(obj);sq_addref(s->vm,&s->refs.back());return (int)s->refs.size()-1;} FAILED}
int csq_push(void* p,int kind,double number,const char* text,size_t size){ENTRY {switch(kind){
    case 0:sq_pushnull(s->vm);break;case 1:sq_pushbool(s->vm,number!=0);break;case 2:sq_pushinteger(s->vm,(SQInteger)number);break;
    case 3:sq_pushfloat(s->vm,number);break;case 4:sq_pushstring(s->vm,text,size);break;case 5:sq_newtable(s->vm);break;case 6:sq_newarray(s->vm,0);break;
    default:return -1;}return 0;} FAILED}
int csq_slot(void* p,int array){ENTRY {return array?sq_arrayappend(s->vm,-2):sq_newslot(s->vm,-3,SQFalse);} FAILED}
}
