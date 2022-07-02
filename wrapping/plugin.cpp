#include <lldb/API/SBCommandInterpreter.h>
#include <lldb/API/SBCommandReturnObject.h>
#include <lldb/API/SBDebugger.h>

namespace lldb {
	bool PluginInitialize(lldb::SBDebugger debugger);
}


extern "C" {
	bool mydbg_search_do_execute(void* debugger, char **command, void* result);
	bool mydbg_read_do_execute(void* debugger, char **command, void* result);
}

class SearchCommand : public lldb::SBCommandPluginInterface {
public:
  virtual bool DoExecute(lldb::SBDebugger debugger, char **command,
                         lldb::SBCommandReturnObject &result) {
	return mydbg_search_do_execute(&debugger, command, &result);
  }
};

class ReadCommand : public lldb::SBCommandPluginInterface {
public:
  virtual bool DoExecute(lldb::SBDebugger debugger, char **command,
                         lldb::SBCommandReturnObject &result) {
	return mydbg_read_do_execute(&debugger, command, &result);
  }
};

bool lldb::PluginInitialize(lldb::SBDebugger debugger) {
  lldb::SBCommandInterpreter interpreter = debugger.GetCommandInterpreter();
  lldb::SBCommand foo = interpreter.AddMultiwordCommand("mydbg", NULL);
  foo.AddCommand("search", new SearchCommand(), "search value from stack/heap/registers");
  foo.AddCommand("read", new ReadCommand(), "read value from memory");
  return true;
}
